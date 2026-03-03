use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::errors::AppError;

const JWT_EXPIRATION_HOURS: i64 = 24;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: i32,
    pub username: String,
    pub role: String,
    pub iat: usize,
    pub exp: usize,
}

pub fn hash_password(password: &str) -> Result<String, AppError> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::InternalError(format!("Failed to hash password: {e}")))
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    bcrypt::verify(password, hash)
        .map_err(|e| AppError::InternalError(format!("Failed to verify password: {e}")))
}

pub fn generate_token(
    user_id: i32,
    username: &str,
    role: &str,
    secret: &str,
) -> Result<String, AppError> {
    let now = chrono::Utc::now().timestamp() as usize;
    let exp = now + (JWT_EXPIRATION_HOURS as usize * 3600);

    let claims = Claims {
        sub: user_id,
        username: username.to_string(),
        role: role.to_string(),
        iat: now,
        exp: exp,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::InternalError(format!("Failed to generate token: {e}")))
}

pub fn validate_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| AppError::Unauthorized(format!("Invalid token: {e}")))?;

    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-secret-key-for-testing";

    #[test]
    fn test_hash_password_produces_valid_hash() {
        let password = "secure_password123";
        let hash = hash_password(password).unwrap();

        assert!(hash.starts_with("$2b$"));
        assert_ne!(hash, password);
    }

    #[test]
    fn test_hash_password_different_each_time() {
        let password = "same_password";
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();

        assert_ne!(hash1, hash2, "Bcrypt should produce different hashes (salt)");
    }

    #[test]
    fn test_verify_password_correct() {
        let password = "my_password_123";
        let hash = hash_password(password).unwrap();

        assert!(verify_password(password, &hash).unwrap());
    }

    #[test]
    fn test_verify_password_wrong() {
        let password = "my_password_123";
        let hash = hash_password(password).unwrap();

        assert!(!verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_generate_and_validate_token() {
        let token = generate_token(1, "john", "admin", TEST_SECRET).unwrap();

        let claims = validate_token(&token, TEST_SECRET).unwrap();
        assert_eq!(claims.sub, 1);
        assert_eq!(claims.username, "john");
        assert_eq!(claims.role, "admin");
    }

    #[test]
    fn test_validate_token_wrong_secret() {
        let token = generate_token(1, "john", "user", TEST_SECRET).unwrap();

        let result = validate_token(&token, "wrong-secret");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_token_invalid_format() {
        let result = validate_token("not.a.valid.token", TEST_SECRET);
        assert!(result.is_err());
    }

    #[test]
    fn test_token_contains_correct_expiration() {
        let token = generate_token(42, "alice", "manager", TEST_SECRET).unwrap();
        let claims = validate_token(&token, TEST_SECRET).unwrap();

        let expected_duration = JWT_EXPIRATION_HOURS as usize * 3600;
        let actual_duration = claims.exp - claims.iat;
        assert_eq!(actual_duration, expected_duration);
    }

    #[test]
    fn test_claims_from_into_authenticated_user() {
        let claims = Claims {
            sub: 5,
            username: "bob".to_string(),
            role: "user".to_string(),
            iat: 0,
            exp: 9999999999,
        };

        let user: crate::middleware::auth::AuthenticatedUser = claims.into();
        assert_eq!(user.user_id, 5);
        assert_eq!(user.username, "bob");
        assert_eq!(user.role, "user");
    }
}
