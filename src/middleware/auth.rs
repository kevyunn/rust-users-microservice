use actix_web::{FromRequest, HttpRequest, dev::Payload, web};
use std::future::{Ready, ready};

use crate::config::JwtSecret;
use crate::errors::AppError;
use crate::services::auth::{Claims, validate_token};

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: i32,
    pub username: String,
    pub role: String,
    pub role_id: i32,
}

impl From<Claims> for AuthenticatedUser {
    fn from(claims: Claims) -> Self {
        Self {
            user_id: claims.sub,
            username: claims.username,
            role: claims.role,
            role_id: claims.role_id,
        }
    }
}

impl FromRequest for AuthenticatedUser {
    type Error = AppError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let token = match extract_bearer_token(req) {
            Ok(t) => t,
            Err(e) => return ready(Err(e)),
        };

        let jwt_secret = match req.app_data::<web::Data<JwtSecret>>() {
            Some(secret) => secret.clone(),
            None => {
                return ready(Err(AppError::InternalError(
                    "JWT secret not configured".into(),
                )));
            }
        };

        match validate_token(&token, &jwt_secret.0) {
            Ok(claims) => ready(Ok(claims.into())),
            Err(e) => ready(Err(e)),
        }
    }
}

fn extract_bearer_token(req: &HttpRequest) -> Result<String, AppError> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".into()))?
        .to_str()
        .map_err(|_| AppError::Unauthorized("Invalid Authorization header".into()))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::Unauthorized(
            "Invalid Authorization scheme, expected Bearer".into(),
        ));
    }

    Ok(auth_header[7..].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, HttpResponse, http::StatusCode, test, web};
    use crate::services::auth::generate_token;

    const TEST_SECRET: &str = "test-secret-for-middleware";

    fn test_app_config(cfg: &mut web::ServiceConfig) {
        cfg.route(
            "/protected",
            web::get().to(|user: AuthenticatedUser| async move {
                HttpResponse::Ok().json(serde_json::json!({
                    "user_id": user.user_id,
                    "username": user.username,
                    "role": user.role
                }))
            }),
        );
    }

    #[actix_web::test]
    async fn test_protected_route_without_token() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(JwtSecret(TEST_SECRET.to_string())))
                .configure(test_app_config),
        )
        .await;

        let req = test::TestRequest::get().uri("/protected").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn test_protected_route_with_invalid_token() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(JwtSecret(TEST_SECRET.to_string())))
                .configure(test_app_config),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("Authorization", "Bearer invalid.token.here"))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn test_protected_route_with_wrong_scheme() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(JwtSecret(TEST_SECRET.to_string())))
                .configure(test_app_config),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("Authorization", "Basic dXNlcjpwYXNz"))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn test_protected_route_with_valid_token() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(JwtSecret(TEST_SECRET.to_string())))
                .configure(test_app_config),
        )
        .await;

        let token = generate_token(1, "john", "admin", 1, TEST_SECRET).unwrap();

        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["user_id"], 1);
        assert_eq!(body["username"], "john");
        assert_eq!(body["role"], "admin");
    }

    #[actix_web::test]
    async fn test_protected_route_with_wrong_secret() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(JwtSecret(TEST_SECRET.to_string())))
                .configure(test_app_config),
        )
        .await;

        let token = generate_token(1, "john", "admin", 1, "different-secret").unwrap();

        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
