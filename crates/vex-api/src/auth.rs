//! JWT-based authentication

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::error::ApiError;

/// JWT claims for API authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user/agent ID)
    pub sub: String,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Issuer
    pub iss: String,
    /// Role (user, agent, admin)
    pub role: String,
    /// Tenant ID (for multi-tenancy)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    /// Custom claims
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

impl Claims {
    /// Create new claims for a user
    pub fn for_user(user_id: &str, role: &str, expires_in: Duration) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id.to_string(),
            exp: (now + expires_in).timestamp(),
            iat: now.timestamp(),
            iss: "vex-api".to_string(),
            role: role.to_string(),
            tenant_id: None,
            extra: std::collections::HashMap::new(),
        }
    }

    /// Create claims for an agent
    pub fn for_agent(agent_id: Uuid, expires_in: Duration) -> Self {
        Self::for_user(&agent_id.to_string(), "agent", expires_in)
    }

    /// Check if claims are expired
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }

    /// Check if claims have a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.role == role || self.role == "admin"
    }
}

/// JWT authentication handler
#[derive(Clone)]
pub struct JwtAuth {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
}

impl JwtAuth {
    /// Create new JWT auth with secret
    pub fn new(secret: &str) -> Self {
        let encoding_key = EncodingKey::from_secret(secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());

        let mut validation = Validation::default();
        validation.set_issuer(&["vex-api"]);
        validation.validate_exp = true;

        Self {
            encoding_key,
            decoding_key,
            validation,
        }
    }

    /// Create from environment variable (required in production)
    /// Uses Zeroizing to securely clear the secret from memory after key creation
    pub fn from_env() -> Result<Self, ApiError> {
        // Wrap secret in Zeroizing to ensure it's cleared from memory when dropped
        let secret: Zeroizing<String> =
            Zeroizing::new(std::env::var("VEX_JWT_SECRET").map_err(|_| {
                ApiError::Internal(
                    "VEX_JWT_SECRET environment variable is required. \
                     Generate with: openssl rand -base64 32"
                        .to_string(),
                )
            })?);

        if secret.len() < 32 {
            return Err(ApiError::Internal(
                "VEX_JWT_SECRET must be at least 32 characters for security".to_string(),
            ));
        }

        // Keys are created, then secret is automatically zeroed when Zeroizing drops
        Ok(Self::new(&secret))
    }

    /// Generate a token for claims
    pub fn encode(&self, claims: &Claims) -> Result<String, ApiError> {
        encode(&Header::default(), claims, &self.encoding_key)
            .map_err(|e| ApiError::Internal(format!("JWT encoding error: {}", e)))
    }

    /// Validate and decode a token
    pub fn decode(&self, token: &str) -> Result<Claims, ApiError> {
        decode::<Claims>(token, &self.decoding_key, &self.validation)
            .map(|data| data.claims)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    ApiError::Unauthorized("Token expired".to_string())
                }
                jsonwebtoken::errors::ErrorKind::InvalidToken => {
                    ApiError::Unauthorized("Invalid token".to_string())
                }
                _ => ApiError::Unauthorized(format!("Token validation failed: {}", e)),
            })
    }

    /// Extract token from Authorization header
    pub fn extract_from_header(header: &str) -> Result<&str, ApiError> {
        header.strip_prefix("Bearer ").ok_or_else(|| {
            ApiError::Unauthorized("Invalid Authorization header format".to_string())
        })
    }
}

/// API key for simplified authentication
#[derive(Debug, Clone)]
pub struct ApiKey {
    pub key_id: uuid::Uuid,
    pub user_id: String,
    pub name: String,
    pub scopes: Vec<String>,
    pub rate_limit: Option<u32>,
}

impl ApiKey {
    /// Validate an API key against a database-backed key store
    /// Uses Argon2id verification with constant-time comparison
    pub async fn validate<S: vex_persist::ApiKeyStore>(
        key: &str,
        store: &S,
    ) -> Result<Self, ApiError> {
        // Use the proper database-backed validation
        let record = vex_persist::validate_api_key(store, key)
            .await
            .map_err(|e| match e {
                vex_persist::ApiKeyError::NotFound => {
                    ApiError::Unauthorized("Invalid API key".to_string())
                }
                vex_persist::ApiKeyError::Expired => {
                    ApiError::Unauthorized("API key expired".to_string())
                }
                vex_persist::ApiKeyError::Revoked => {
                    ApiError::Unauthorized("API key revoked".to_string())
                }
                vex_persist::ApiKeyError::InvalidFormat => {
                    ApiError::Unauthorized("Invalid API key format".to_string())
                }
                vex_persist::ApiKeyError::Storage(msg) => {
                    ApiError::Internal(format!("Key validation error: {}", msg))
                }
            })?;

        // Determine rate limit based on scopes
        let rate_limit = if record.scopes.contains(&"enterprise".to_string()) {
            Some(10000)
        } else if record.scopes.contains(&"pro".to_string()) {
            Some(1000)
        } else {
            Some(100) // Free tier default
        };

        Ok(ApiKey {
            key_id: record.id,
            user_id: record.user_id,
            name: record.name,
            scopes: record.scopes,
            rate_limit,
        })
    }

    /// Check if this API key has a specific scope
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope || s == "*")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_encode_decode() {
        let auth = JwtAuth::new("test-secret-key-32-bytes-long!!");
        let claims = Claims::for_user("user123", "user", Duration::hours(1));

        let token = auth.encode(&claims).unwrap();
        let decoded = auth.decode(&token).unwrap();

        assert_eq!(decoded.sub, "user123");
        assert_eq!(decoded.role, "user");
        assert!(!decoded.is_expired());
    }

    #[test]
    fn test_expired_token() {
        let auth = JwtAuth::new("test-secret-key-32-bytes-long!!");
        // Use -300s to ensure we exceed default 60s leeway
        let claims = Claims::for_user("user123", "user", Duration::seconds(-300));

        let token = auth.encode(&claims).unwrap();
        let result = auth.decode(&token);

        match &result {
            Ok(c) => println!("Decoded claims despite expiry: {:?}", c),
            Err(e) => println!("Error returned: {:?}", e),
        }

        assert!(
            matches!(result, Err(ApiError::Unauthorized(_))),
            "Expected Unauthorized error, got: {:?}",
            result
        );
    }

    #[test]
    fn test_role_check() {
        let claims = Claims::for_user("user123", "admin", Duration::hours(1));
        assert!(claims.has_role("admin"));
        assert!(claims.has_role("user")); // Admin has all roles
    }
}
