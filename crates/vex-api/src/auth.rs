//! JWT-based authentication

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    pub fn from_env() -> Result<Self, ApiError> {
        let secret = std::env::var("VEX_JWT_SECRET").map_err(|_| {
            ApiError::Internal(
                "VEX_JWT_SECRET environment variable is required. \
                     Generate with: openssl rand -base64 32"
                    .to_string(),
            )
        })?;

        if secret.len() < 32 {
            return Err(ApiError::Internal(
                "VEX_JWT_SECRET must be at least 32 characters for security".to_string(),
            ));
        }

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
    pub key: String,
    pub name: String,
    pub roles: Vec<String>,
    pub rate_limit: Option<u32>,
}

impl ApiKey {
    /// Validate an API key (placeholder - connect to your key store)
    pub async fn validate(key: &str) -> Result<Self, ApiError> {
        // In production, this would check against a database
        if key.starts_with("vex_") && key.len() > 20 {
            Ok(ApiKey {
                key: key.to_string(),
                name: "default".to_string(),
                roles: vec!["user".to_string()],
                rate_limit: Some(100),
            })
        } else {
            Err(ApiError::Unauthorized("Invalid API key".to_string()))
        }
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
