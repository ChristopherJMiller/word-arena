use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json;
use tokio::sync::RwLock;

use game_types::User;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrosoftJwtClaims {
    pub aud: String,                // Audience
    pub iss: String,                // Issuer
    pub iat: u64,                   // Issued at
    pub exp: u64,                   // Expiry
    pub sub: Option<String>,        // Subject (user ID) - optional
    pub oid: Option<String>,        // Object ID (Azure AD user ID) - optional
    pub email: Option<String>,      // User email - optional in some scenarios
    pub name: Option<String>,       // Display name - optional
    pub preferred_username: Option<String>, // Username - optional
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
        let header = decode_header(token).map_err(|e| {
            tracing::warn!("Failed to decode JWT header: {:?}", e);
            AuthError::InvalidToken
        })?;
        let kid = header.kid.ok_or_else(|| {
            tracing::warn!("JWT header missing 'kid' field");
            AuthError::InvalidToken
        })?;

        // Get or fetch the public key
        tracing::debug!("Fetching decoding key for kid: {}", kid);
        let decoding_key = self.get_decoding_key(&kid).await?;

        // Validate the token
        let mut validation = Validation::new(Algorithm::RS256);
        // Use our app's client ID as the audience (not Microsoft Graph)
        validation.set_audience(&[&self.client_id]);
        
        // Handle issuer validation based on tenant type
        let is_common_tenant = self.tenant_id == "common";
        if is_common_tenant {
            // For common tenant, use dangerous validation without issuer check
            validation = Validation::new(Algorithm::RS256);
            validation.set_audience(&[&self.client_id]);
            validation.validate_exp = true;
            validation.validate_nbf = true;
            validation.validate_aud = true;
            // Don't set issuer - will manually validate
            tracing::debug!("Using common tenant - will manually validate Microsoft issuer");
        } else {
            // For specific tenant, validate against expected issuer formats
            let v1_issuer = format!("https://sts.windows.net/{}/", self.tenant_id);
            let v2_issuer = format!("https://login.microsoftonline.com/{}/v2.0", self.tenant_id);
            validation.set_issuer(&[&v1_issuer, &v2_issuer]);
            tracing::debug!("Accepted issuers: {} and {}", v1_issuer, v2_issuer);
        }
        
        tracing::debug!("Validating token with audience: {}", self.client_id);

        let token_data = decode::<MicrosoftJwtClaims>(token, &decoding_key, &validation)
            .map_err(|e| {
                tracing::warn!("JWT token validation failed: {:?}", e);
                tracing::warn!("Token validation details:");
                tracing::warn!("  - Algorithm: RS256 (expected)");
                tracing::warn!("  - Audience: 00000003-0000-0000-c000-000000000000 (expected)");
                tracing::warn!("  - Issuer: accepting both v1.0 and v2.0 formats");
                tracing::warn!("  - Key ID: {} (found in JWKS)", kid);
                tracing::warn!("This could indicate:");
                tracing::warn!("  1. Token signature is invalid/corrupted");
                tracing::warn!("  2. Token was signed with a different key");
                tracing::warn!("  3. Token format/encoding issue");
                AuthError::InvalidToken
            })?;

        let claims = token_data.claims;

        tracing::debug!("Token claims - aud: {}, iss: {}", claims.aud, claims.iss);
        if let Some(ref oid) = claims.oid {
            tracing::debug!("User oid: {}", oid);
        }

        // Manual issuer validation for common tenant
        if is_common_tenant {
            let valid_issuer = claims.iss.starts_with("https://sts.windows.net/") || 
                               claims.iss.starts_with("https://login.microsoftonline.com/");
            if !valid_issuer {
                tracing::warn!("Invalid issuer for common tenant: {}", claims.iss);
                return Err(AuthError::InvalidToken);
            }
            tracing::debug!("Valid Microsoft issuer: {}", claims.iss);
        }

        // Verify token is not expired
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if claims.exp < now {
            tracing::warn!("Token expired: exp={}, now={}", claims.exp, now);
            return Err(AuthError::TokenExpired);
        }

        // Create user from claims
        // Use oid (object ID) if available, fallback to sub, then generate new UUID
        let user_id = claims.oid
            .or(claims.sub)
            .and_then(|id| uuid::Uuid::parse_str(&id).ok())
            .unwrap_or_else(|| uuid::Uuid::new_v4());

        Ok(User {
            id: user_id,
            email: claims.email.unwrap_or_else(|| "unknown@example.com".to_string()),
            display_name: claims.name.unwrap_or_else(|| "Unknown User".to_string()),
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
                let elapsed = cached_time.elapsed().unwrap_or(Duration::from_secs(3600));
                if elapsed < Duration::from_secs(3600) {
                    tracing::debug!("Using cached decoding key for kid '{}' (cached {}s ago)", kid, elapsed.as_secs());
                    return Ok(key.clone());
                } else {
                    tracing::debug!("Cached key for kid '{}' is expired ({}s old), fetching fresh", kid, elapsed.as_secs());
                }
            } else {
                tracing::debug!("No cached key found for kid '{}', fetching from JWKS", kid);
            }
        }

        // Fetch from Microsoft
        let jwks_url = format!(
            "https://login.microsoftonline.com/{}/discovery/v2.0/keys",
            self.tenant_id
        );
        tracing::debug!("Fetching JWKS from tenant {} at: {}", self.tenant_id, jwks_url);

        let response = self
            .client
            .get(&jwks_url)
            .send()
            .await
            .map_err(|e| {
                tracing::warn!("Failed to fetch JWKS: {:?}", e);
                AuthError::JwksFetchError
            })?;

        if !response.status().is_success() {
            tracing::warn!("JWKS fetch returned status: {}", response.status());
            return Err(AuthError::JwksFetchError);
        }

        let jwks: JwksResponse = response
            .json()
            .await
            .map_err(|e| {
                tracing::warn!("Failed to parse JWKS JSON: {:?}", e);
                AuthError::JwksFetchError
            })?;

        tracing::debug!("Successfully fetched JWKS with {} keys", jwks.keys.len());

        // Find the key with matching kid
        tracing::debug!("Looking for key with kid: {}", kid);
        tracing::debug!("Available keys: {:?}", jwks.keys.iter().map(|k| &k.kid).collect::<Vec<_>>());
        
        let jwks_key = jwks
            .keys
            .iter()
            .find(|key| key.kid == kid)
            .ok_or_else(|| {
                tracing::warn!("Key with kid '{}' not found in JWKS from tenant {}", kid, self.tenant_id);
                tracing::warn!("This means the token was signed by a different tenant or the key has rotated");
                AuthError::KeyNotFound
            })?;
            
        tracing::debug!("Found matching key with kid '{}' in JWKS from tenant {}", kid, self.tenant_id);

        // Convert to DecodingKey
        tracing::debug!("Converting JWKS key to decoding key. Has n,e: {}, Has x5c: {}", 
                       jwks_key.n.is_some() && jwks_key.e.is_some(),
                       jwks_key.x5c.is_some());
        
        let decoding_key = if let (Some(n), Some(e)) = (&jwks_key.n, &jwks_key.e) {
            tracing::debug!("Using RSA components (n,e) to create decoding key");
            DecodingKey::from_rsa_components(n, e).map_err(|e| {
                tracing::warn!("Failed to create decoding key from RSA components: {:?}", e);
                AuthError::InvalidKey
            })?
        } else if let Some(x5c) = &jwks_key.x5c {
            if let Some(cert) = x5c.first() {
                tracing::debug!("Using x5c certificate to create decoding key");
                let cert_der = base64::engine::general_purpose::STANDARD
                    .decode(cert)
                    .map_err(|e| {
                        tracing::warn!("Failed to decode x5c certificate: {:?}", e);
                        AuthError::InvalidKey
                    })?;
                DecodingKey::from_rsa_der(&cert_der)
            } else {
                tracing::warn!("x5c array is empty");
                return Err(AuthError::InvalidKey);
            }
        } else {
            tracing::warn!("JWKS key has neither n,e components nor x5c certificate");
            return Err(AuthError::InvalidKey);
        };
        
        tracing::debug!("Successfully created decoding key for kid: {}", kid);

        // Cache the key
        {
            let mut cache = self.jwks_cache.write().await;
            cache.insert(kid.to_string(), (decoding_key.clone(), SystemTime::now()));
        }

        Ok(decoding_key)
    }


    async fn validate_dev_token(&self, token: &str) -> Result<User, AuthError> {
        // In dev mode, we expect a JWT-like token but we parse it without validation
        // We just decode the payload section and extract the claims
        tracing::debug!("Validating dev token (first 20 chars): {}", &token[..token.len().min(20)]);

        // Check if it looks like a JWT (has 3 parts separated by dots)
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() == 3 {
            // Decode the payload (second part)
            let payload_b64 = parts[1];

            // Add padding if needed for base64 decoding
            let padded_payload = match payload_b64.len() % 4 {
                0 => payload_b64.to_string(),
                n => format!("{}{}", payload_b64, "=".repeat(4 - n)),
            };

            // Convert URL-safe base64 back to standard base64
            let standard_b64 = padded_payload.replace('-', "+").replace('_', "/");

            // Decode base64
            let payload_bytes =
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, standard_b64)
                    .map_err(|e| {
                        tracing::warn!("Failed to decode JWT payload in dev mode: {:?}", e);
                        AuthError::InvalidToken
                    })?;

            // Parse as JSON to get claims
            let claims: MicrosoftJwtClaims =
                serde_json::from_slice(&payload_bytes).map_err(|e| {
                    tracing::warn!("Failed to parse JWT claims in dev mode: {:?}", e);
                    AuthError::InvalidToken
                })?;

            // Create user from claims (no validation in dev mode)
            // Use oid (object ID) if available, fallback to sub, then generate new UUID
            let user_id = claims.oid
                .or(claims.sub)
                .and_then(|id| uuid::Uuid::parse_str(&id).ok())
                .unwrap_or_else(|| uuid::Uuid::new_v4());

            Ok(User {
                id: user_id,
                email: claims.email.unwrap_or_else(|| "dev@example.com".to_string()),
                display_name: claims.name.unwrap_or_else(|| "Dev User".to_string()),
                total_points: 0,
                total_wins: 0,
                total_games: 0,
                created_at: chrono::Utc::now().to_string(),
            })
        } else {
            // Fallback for non-JWT format (for backwards compatibility)
            if token.starts_with("{") && token.ends_with("}") {
                // Parse as JSON
                #[derive(serde::Deserialize)]
                struct DevClaims {
                    user_id: String,
                    email: String,
                    name: String,
                }

                let claims: DevClaims =
                    serde_json::from_str(token).map_err(|_| AuthError::InvalidToken)?;

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
                let string_parts: Vec<&str> = token.split(':').collect();
                if string_parts.len() >= 3 {
                    Ok(User {
                        id: uuid::Uuid::parse_str(string_parts[0])
                            .unwrap_or_else(|_| uuid::Uuid::new_v4()),
                        email: string_parts[1].to_string(),
                        display_name: string_parts[2].to_string(),
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
        let auth_service = AuthService::new("test-tenant".to_string(), "test-client".to_string());

        assert_eq!(auth_service.tenant_id, "test-tenant");
        assert_eq!(auth_service.client_id, "test-client");
    }

    #[tokio::test]
    async fn test_invalid_token_validation() {
        let auth_service = AuthService::new("test-tenant".to_string(), "test-client".to_string());

        let result = auth_service.validate_token("invalid-token").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::InvalidToken));
    }
}
