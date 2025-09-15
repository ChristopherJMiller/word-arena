use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json;
use tokio::sync::RwLock;
use uuid::Uuid;

use game_types::User;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrosoftJwtClaims {
    pub aud: String,           // Audience
    pub iss: String,           // Issuer
    pub iat: u64,              // Issued at
    pub exp: u64,              // Expiry
    pub sub: String,           // Subject (user ID)
    pub email: String,         // User email
    pub name: String,          // Display name
    pub preferred_username: String, // Username
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwksKey {
    pub kty: String,
    pub use_: Option<String>,
    #[serde(rename = "use")]
    pub use_field: Option<String>,
    pub x5c: Option<Vec<String>>,
    pub n: Option<String>,
    pub e: Option<String>,
    pub kid: String,
    pub x5t: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwksResponse {
    pub keys: Vec<JwksKey>,
}

pub struct AuthService {
    client: Client,
    jwks_cache: Arc<RwLock<HashMap<String, (DecodingKey, SystemTime)>>>,
    tenant_id: String,
    client_id: String,
    dev_mode: bool,
}

impl AuthService {
    pub fn new(tenant_id: String, client_id: String) -> Self {
        Self {
            client: Client::new(),
            jwks_cache: Arc::new(RwLock::new(HashMap::new())),
            tenant_id,
            client_id,
            dev_mode: false,
        }
    }

    pub fn new_dev_mode() -> Self {
        Self {
            client: Client::new(),
            jwks_cache: Arc::new(RwLock::new(HashMap::new())),
            tenant_id: "dev".to_string(),
            client_id: "dev".to_string(),
            dev_mode: true,
        }
    }

    pub async fn validate_token(&self, token: &str) -> Result<User, AuthError> {
        if self.dev_mode {
            return self.validate_dev_token(token).await;
        }

        // Decode header to get key ID
        let header = decode_header(token).map_err(|_| AuthError::InvalidToken)?;
        let kid = header.kid.ok_or(AuthError::InvalidToken)?;

        // Get or fetch the public key
        let decoding_key = self.get_decoding_key(&kid).await?;

        // Validate the token
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[&self.client_id]);
        validation.set_issuer(&[&format!("https://login.microsoftonline.com/{}/v2.0", self.tenant_id)]);

        let token_data = decode::<MicrosoftJwtClaims>(token, &decoding_key, &validation)
            .map_err(|_| AuthError::InvalidToken)?;

        let claims = token_data.claims;

        // Verify token is not expired
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if claims.exp < now {
            return Err(AuthError::TokenExpired);
        }

        // Create user from claims
        Ok(User {
            id: Uuid::new_v4(), // We'll need to look this up or create in database
            email: claims.email,
            display_name: claims.name,
            total_points: 0,
            total_wins: 0,
            total_games: 0,
            created_at: chrono::Utc::now().to_string(),
        })
    }

    async fn get_decoding_key(&self, kid: &str) -> Result<DecodingKey, AuthError> {
        // Check cache first
        {
            let cache = self.jwks_cache.read().await;
            if let Some((key, cached_time)) = cache.get(kid) {
                // Cache for 1 hour
                if cached_time.elapsed().unwrap_or(Duration::from_secs(3600)) < Duration::from_secs(3600) {
                    return Ok(key.clone());
                }
            }
        }

        // Fetch from Microsoft
        let jwks_url = format!(
            "https://login.microsoftonline.com/{}/discovery/v2.0/keys",
            self.tenant_id
        );

        let response = self
            .client
            .get(&jwks_url)
            .send()
            .await
            .map_err(|_| AuthError::JwksFetchError)?;

        let jwks: JwksResponse = response
            .json()
            .await
            .map_err(|_| AuthError::JwksFetchError)?;

        // Find the key with matching kid
        let jwks_key = jwks
            .keys
            .iter()
            .find(|key| key.kid == kid)
            .ok_or(AuthError::KeyNotFound)?;

        // Convert to DecodingKey
        let decoding_key = if let (Some(n), Some(e)) = (&jwks_key.n, &jwks_key.e) {
            DecodingKey::from_rsa_components(n, e).map_err(|_| AuthError::InvalidKey)?
        } else if let Some(x5c) = &jwks_key.x5c {
            if let Some(cert) = x5c.first() {
                let cert_der = base64::engine::general_purpose::STANDARD.decode(cert).map_err(|_| AuthError::InvalidKey)?;
                // from_rsa_der doesn't return a Result, it's infallible for valid DER
                DecodingKey::from_rsa_der(&cert_der)
            } else {
                return Err(AuthError::InvalidKey);
            }
        } else {
            return Err(AuthError::InvalidKey);
        };

        // Cache the key
        {
            let mut cache = self.jwks_cache.write().await;
            cache.insert(kid.to_string(), (decoding_key.clone(), SystemTime::now()));
        }

        Ok(decoding_key)
    }

    async fn validate_dev_token(&self, token: &str) -> Result<User, AuthError> {
        // In dev mode, we expect a simple JSON payload instead of a JWT
        // Format: {"user_id":"123","email":"test@example.com","name":"Test User"}
        
        if token.starts_with("{") && token.ends_with("}") {
            // Parse as JSON
            #[derive(serde::Deserialize)]
            struct DevClaims {
                user_id: String,
                email: String,
                name: String,
            }

            let claims: DevClaims = serde_json::from_str(token)
                .map_err(|_| AuthError::InvalidToken)?;

            Ok(User {
                id: uuid::Uuid::parse_str(&claims.user_id)
                    .unwrap_or_else(|_| uuid::Uuid::new_v4()),
                email: claims.email,
                display_name: claims.name,
                total_points: 0,
                total_wins: 0,
                total_games: 0,
                created_at: chrono::Utc::now().to_string(),
            })
        } else {
            // Simple string format: "user_id:email:name"
            let parts: Vec<&str> = token.split(':').collect();
            if parts.len() >= 3 {
                Ok(User {
                    id: uuid::Uuid::parse_str(parts[0])
                        .unwrap_or_else(|_| uuid::Uuid::new_v4()),
                    email: parts[1].to_string(),
                    display_name: parts[2].to_string(),
                    total_points: 0,
                    total_wins: 0,
                    total_games: 0,
                    created_at: chrono::Utc::now().to_string(),
                })
            } else {
                Err(AuthError::InvalidToken)
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid token")]
    InvalidToken,
    #[error("Token expired")]
    TokenExpired,
    #[error("Failed to fetch JWKS")]
    JwksFetchError,
    #[error("Key not found")]
    KeyNotFound,
    #[error("Invalid key")]
    InvalidKey,
    #[error("Audience mismatch")]
    AudienceMismatch,
    #[error("Issuer mismatch")]
    IssuerMismatch,
}

impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(_: jsonwebtoken::errors::Error) -> Self {
        AuthError::InvalidKey
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_service_creation() {
        let auth_service = AuthService::new(
            "test-tenant".to_string(),
            "test-client".to_string(),
        );
        
        assert_eq!(auth_service.tenant_id, "test-tenant");
        assert_eq!(auth_service.client_id, "test-client");
    }

    #[tokio::test]
    async fn test_invalid_token_validation() {
        let auth_service = AuthService::new(
            "test-tenant".to_string(),
            "test-client".to_string(),
        );

        let result = auth_service.validate_token("invalid-token").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::InvalidToken));
    }
}