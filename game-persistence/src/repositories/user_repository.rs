use anyhow::Result;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect,
};
use uuid::Uuid;

use crate::entities::{prelude::*, users};
use game_types::User;

pub struct UserRepository {
    db: DatabaseConnection,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LeaderboardEntry {
    pub user: User,
    pub rank: u32,
}

impl UserRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    fn model_to_user(model: users::Model) -> User {
        User {
            id: model.id,
            email: model.email,
            display_name: model.display_name,
            total_points: model.total_points,
            total_wins: model.total_wins,
            total_games: model.total_games,
            created_at: model.created_at.to_rfc3339(),
        }
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let user_model = Users::find_by_id(id).one(&self.db).await?;
        Ok(user_model.map(Self::model_to_user))
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        let user_model = Users::find()
            .filter(users::Column::Email.eq(email))
            .one(&self.db)
            .await?;

        Ok(user_model.map(Self::model_to_user))
    }

    pub async fn create_user(&self, user: User) -> Result<User> {
        let now = chrono::Utc::now().into();
        let created_at = chrono::DateTime::parse_from_rfc3339(&user.created_at)
            .unwrap_or_else(|_| chrono::Utc::now().into());

        let user_model = users::ActiveModel {
            id: sea_orm::ActiveValue::Set(user.id),
            email: sea_orm::ActiveValue::Set(user.email),
            display_name: sea_orm::ActiveValue::Set(user.display_name),
            total_points: sea_orm::ActiveValue::Set(user.total_points),
            total_wins: sea_orm::ActiveValue::Set(user.total_wins),
            total_games: sea_orm::ActiveValue::Set(user.total_games),
            created_at: sea_orm::ActiveValue::Set(created_at),
            updated_at: sea_orm::ActiveValue::Set(now),
        };

        let saved_model = Users::insert(user_model).exec(&self.db).await?;

        // Fetch the created user
        let created_user = Users::find_by_id(saved_model.last_insert_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve created user"))?;

        Ok(Self::model_to_user(created_user))
    }

    pub async fn update_user_stats(
        &self,
        user_id: Uuid,
        points_gained: i32,
        won: bool,
    ) -> Result<()> {
        let user = Users::find_by_id(user_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;

        let updated_user = users::ActiveModel {
            id: sea_orm::ActiveValue::Unchanged(user.id),
            email: sea_orm::ActiveValue::Unchanged(user.email),
            display_name: sea_orm::ActiveValue::Unchanged(user.display_name),
            total_points: sea_orm::ActiveValue::Set(user.total_points + points_gained),
            total_wins: sea_orm::ActiveValue::Set(user.total_wins + if won { 1 } else { 0 }),
            total_games: sea_orm::ActiveValue::Set(user.total_games + 1),
            created_at: sea_orm::ActiveValue::Unchanged(user.created_at),
            updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now().into()),
        };

        Users::update(updated_user).exec(&self.db).await?;
        Ok(())
    }

    pub async fn get_leaderboard(&self, limit: u64) -> Result<Vec<LeaderboardEntry>> {
        let users = Users::find()
            .order_by_desc(users::Column::TotalPoints)
            .limit(limit)
            .all(&self.db)
            .await?;

        let leaderboard = users
            .into_iter()
            .enumerate()
            .map(|(index, model)| LeaderboardEntry {
                user: Self::model_to_user(model),
                rank: (index + 1) as u32,
            })
            .collect();

        Ok(leaderboard)
    }

    pub async fn get_user_rank(&self, user_id: Uuid) -> Result<Option<u32>> {
        let user = Users::find_by_id(user_id).one(&self.db).await?;

        if let Some(user_model) = user {
            let users_above = Users::find()
                .filter(users::Column::TotalPoints.gt(user_model.total_points))
                .count(&self.db)
                .await?;

            Ok(Some(users_above as u32 + 1))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::connect_to_memory_database;
    use migration::{Migrator, MigratorTrait};
    use uuid::Uuid;

    async fn setup_test_db() -> UserRepository {
        let db = connect_to_memory_database().await.unwrap();
        Migrator::up(&db, None).await.unwrap();
        UserRepository::new(db)
    }

    #[tokio::test]
    async fn test_create_and_find_user() {
        let repo = setup_test_db().await;

        let user_id = Uuid::new_v4();
        let user = User {
            id: user_id,
            email: "test@example.com".to_string(),
            display_name: "Test User".to_string(),
            total_points: 0,
            total_wins: 0,
            total_games: 0,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        // Create user
        let created_user = repo.create_user(user.clone()).await.unwrap();
        assert_eq!(created_user.email, user.email);
        assert_eq!(created_user.display_name, user.display_name);

        // Find by ID
        let found_user = repo.find_by_id(user_id).await.unwrap().unwrap();
        assert_eq!(found_user.email, user.email);

        // Find by email
        let found_by_email = repo.find_by_email(&user.email).await.unwrap().unwrap();
        assert_eq!(found_by_email.id, user_id);
    }

    #[tokio::test]
    async fn test_update_user_stats() {
        let repo = setup_test_db().await;

        let user_id = Uuid::new_v4();
        let user = User {
            id: user_id,
            email: "test@example.com".to_string(),
            display_name: "Test User".to_string(),
            total_points: 10,
            total_wins: 1,
            total_games: 2,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        repo.create_user(user).await.unwrap();

        // Update stats (won game with 5 points)
        repo.update_user_stats(user_id, 5, true).await.unwrap();

        let updated_user = repo.find_by_id(user_id).await.unwrap().unwrap();
        assert_eq!(updated_user.total_points, 15);
        assert_eq!(updated_user.total_wins, 2);
        assert_eq!(updated_user.total_games, 3);
    }

    #[tokio::test]
    async fn test_leaderboard() {
        let repo = setup_test_db().await;

        // Create test users with different scores
        let users = vec![
            User {
                id: Uuid::new_v4(),
                email: "user1@example.com".to_string(),
                display_name: "User One".to_string(),
                total_points: 100,
                total_wins: 5,
                total_games: 10,
                created_at: chrono::Utc::now().to_rfc3339(),
            },
            User {
                id: Uuid::new_v4(),
                email: "user2@example.com".to_string(),
                display_name: "User Two".to_string(),
                total_points: 200,
                total_wins: 8,
                total_games: 12,
                created_at: chrono::Utc::now().to_rfc3339(),
            },
            User {
                id: Uuid::new_v4(),
                email: "user3@example.com".to_string(),
                display_name: "User Three".to_string(),
                total_points: 50,
                total_wins: 2,
                total_games: 8,
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        ];

        // Create all users
        for user in &users {
            repo.create_user(user.clone()).await.unwrap();
        }

        // Get leaderboard
        let leaderboard = repo.get_leaderboard(10).await.unwrap();

        assert_eq!(leaderboard.len(), 3);

        // Should be ordered by points descending
        assert_eq!(leaderboard[0].user.total_points, 200);
        assert_eq!(leaderboard[0].rank, 1);
        assert_eq!(leaderboard[0].user.display_name, "User Two");

        assert_eq!(leaderboard[1].user.total_points, 100);
        assert_eq!(leaderboard[1].rank, 2);

        assert_eq!(leaderboard[2].user.total_points, 50);
        assert_eq!(leaderboard[2].rank, 3);
    }

    #[tokio::test]
    async fn test_user_rank() {
        let repo = setup_test_db().await;

        let users = vec![
            User {
                id: Uuid::new_v4(),
                email: "user1@example.com".to_string(),
                display_name: "User One".to_string(),
                total_points: 100,
                total_wins: 5,
                total_games: 10,
                created_at: chrono::Utc::now().to_rfc3339(),
            },
            User {
                id: Uuid::new_v4(),
                email: "user2@example.com".to_string(),
                display_name: "User Two".to_string(),
                total_points: 200,
                total_wins: 8,
                total_games: 12,
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        ];

        for user in &users {
            repo.create_user(user.clone()).await.unwrap();
        }

        // User with 200 points should be rank 1
        let rank = repo.get_user_rank(users[1].id).await.unwrap().unwrap();
        assert_eq!(rank, 1);

        // User with 100 points should be rank 2
        let rank = repo.get_user_rank(users[0].id).await.unwrap().unwrap();
        assert_eq!(rank, 2);

        // Non-existent user should return None
        let rank = repo.get_user_rank(Uuid::new_v4()).await.unwrap();
        assert_eq!(rank, None);
    }

    #[tokio::test]
    async fn test_leaderboard_limit() {
        let repo = setup_test_db().await;

        // Create 5 users
        for i in 1..=5 {
            let user = User {
                id: Uuid::new_v4(),
                email: format!("user{}@example.com", i),
                display_name: format!("User {}", i),
                total_points: i * 10,
                total_wins: i,
                total_games: i * 2,
                created_at: chrono::Utc::now().to_rfc3339(),
            };
            repo.create_user(user).await.unwrap();
        }

        // Get top 3
        let leaderboard = repo.get_leaderboard(3).await.unwrap();
        assert_eq!(leaderboard.len(), 3);

        // Should be in descending order by points
        assert_eq!(leaderboard[0].user.total_points, 50);
        assert_eq!(leaderboard[1].user.total_points, 40);
        assert_eq!(leaderboard[2].user.total_points, 30);
    }
}
