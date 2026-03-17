use actix_web::{web, HttpResponse};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::db::DbPool;
use crate::errors::{AppError, ErrorResponse};
use crate::middleware::auth::AuthenticatedUser;
use crate::models::role::Role;
use crate::models::user::{User, UserResponse};
use crate::schema::{roles, users};
use crate::services::role as role_service;
use crate::services::user as user_service;
use diesel::prelude::*;

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub password: Option<String>,
    pub role_id: Option<i32>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, serde::Serialize)]
pub struct PaginatedResponse<T: serde::Serialize> {
    pub data: T,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

/// Response for GET /api/users (for OpenAPI schema)
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct PaginatedUsersResponse {
    pub data: Vec<crate::models::user::UserResponse>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[utoipa::path(
    get,
    path = "/api/users/me",
    responses(
        (status = 200, description = "Current user", body = UserResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse)
    ),
    security((), ("bearer_auth" = []))
)]
pub async fn me(
    pool: web::Data<DbPool>,
    auth_user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    let pool = pool.into_inner();
    let role_id = auth_user.role_id;
    let user_id = auth_user.user_id;

    web::block({
        let pool = pool.clone();
        move || role_service::require_permission(&pool, role_id, "users", "read_own")
    })
    .await??;

    let user_response = web::block(move || {
        let mut conn = pool.get().map_err(|e| AppError::DbError(e.to_string()))?;

        let user: User = users::table.find(user_id).first(&mut conn)?;
        let role: Role = roles::table.find(user.role_id).first(&mut conn)?;

        Ok::<UserResponse, AppError>(UserResponse::from_user(user, role.name))
    })
    .await??;

    Ok(HttpResponse::Ok().json(user_response))
}

#[utoipa::path(
    get,
    path = "/api/users",
    params(
        ("page" = Option<i64>, Query, description = "Page number"),
        ("per_page" = Option<i64>, Query, description = "Items per page")
    ),
    responses(
        (status = 200, description = "Paginated users", body = PaginatedUsersResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse)
    ),
    security((), ("bearer_auth" = []))
)]
pub async fn list(
    pool: web::Data<DbPool>,
    auth_user: AuthenticatedUser,
    query: web::Query<PaginationParams>,
) -> Result<HttpResponse, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let pool = pool.into_inner();
    let role_id = auth_user.role_id;

    web::block({
        let pool = pool.clone();
        move || role_service::require_permission(&pool, role_id, "users", "read")
    })
    .await??;

    let (users, total) =
        web::block(move || user_service::list_users(&pool, page, per_page)).await??;

    Ok(HttpResponse::Ok().json(PaginatedResponse {
        data: users,
        total,
        page,
        per_page,
    }))
}

#[utoipa::path(
    get,
    path = "/api/users/{id}",
    params(("id" = i32, Path, description = "User ID")),
    responses(
        (status = 200, description = "User", body = UserResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse)
    ),
    security((), ("bearer_auth" = []))
)]
pub async fn get_by_id(
    pool: web::Data<DbPool>,
    auth_user: AuthenticatedUser,
    path: web::Path<i32>,
) -> Result<HttpResponse, AppError> {
    let target_id = path.into_inner();
    let pool = pool.into_inner();
    let role_id = auth_user.role_id;
    let action = if target_id == auth_user.user_id {
        "read_own"
    } else {
        "read"
    };

    web::block({
        let pool = pool.clone();
        move || role_service::require_permission(&pool, role_id, "users", action)
    })
    .await??;

    let user_response =
        web::block(move || user_service::find_user_by_id(&pool, target_id)).await??;

    Ok(HttpResponse::Ok().json(user_response))
}

#[utoipa::path(
    put,
    path = "/api/users/{id}",
    params(("id" = i32, Path, description = "User ID")),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "Updated user", body = UserResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse)
    ),
    security((), ("bearer_auth" = []))
)]
pub async fn update(
    pool: web::Data<DbPool>,
    auth_user: AuthenticatedUser,
    path: web::Path<i32>,
    body: web::Json<UpdateUserRequest>,
) -> Result<HttpResponse, AppError> {
    let target_id = path.into_inner();
    let pool = pool.into_inner();
    let role_id = auth_user.role_id;
    let body = body.into_inner();

    let (required_action, allow_role_or_active) = if target_id == auth_user.user_id {
        ("update_own", false)
    } else {
        ("update", true)
    };

    web::block({
        let pool = pool.clone();
        move || role_service::require_permission(&pool, role_id, "users", required_action)
    })
    .await??;

    let can_change_role_or_active = allow_role_or_active
        && web::block({
            let pool = pool.clone();
            move || role_service::require_permission(&pool, role_id, "users", "update")
        })
        .await
        .ok()
        .and_then(Result::ok)
        .is_some();

    if (body.role_id.is_some() || body.is_active.is_some()) && !can_change_role_or_active {
        return Err(AppError::Forbidden(
            "Only users with users:update can change role or active status".into(),
        ));
    }

    let user_response = web::block(move || {
        user_service::update_user(
            &pool,
            target_id,
            body.email,
            body.first_name,
            body.last_name,
            body.password,
            if can_change_role_or_active {
                body.role_id
            } else {
                None
            },
            if can_change_role_or_active {
                body.is_active
            } else {
                None
            },
            can_change_role_or_active,
        )
    })
    .await??;

    Ok(HttpResponse::Ok().json(user_response))
}

#[utoipa::path(
    delete,
    path = "/api/users/{id}",
    params(("id" = i32, Path, description = "User ID")),
    responses(
        (status = 204, description = "Deactivated"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse)
    ),
    security((), ("bearer_auth" = []))
)]
pub async fn delete(
    pool: web::Data<DbPool>,
    auth_user: AuthenticatedUser,
    path: web::Path<i32>,
) -> Result<HttpResponse, AppError> {
    let target_id = path.into_inner();
    let pool = pool.into_inner();
    let role_id = auth_user.role_id;

    web::block({
        let pool = pool.clone();
        move || role_service::require_permission(&pool, role_id, "users", "delete")
    })
    .await??;

    web::block(move || user_service::deactivate_user(&pool, target_id)).await??;

    Ok(HttpResponse::NoContent().finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, http::StatusCode, test};
    use crate::config::JwtSecret;
    use crate::db::DbPool;
    use crate::services::auth::generate_token;
    use diesel::r2d2::{self, ConnectionManager};
    use diesel::pg::PgConnection;

    const TEST_SECRET: &str = "test-secret-for-users";

    fn dummy_pool() -> DbPool {
        let manager = ConnectionManager::<PgConnection>::new("postgres://fake@localhost/fake");
        r2d2::Pool::builder()
            .max_size(1)
            .build_unchecked(manager)
    }

    fn test_app_config(cfg: &mut web::ServiceConfig) {
        cfg.service(
            web::scope("/api/users")
                .route("/me", web::get().to(me))
                .route("", web::get().to(list))
                .route("/{id}", web::get().to(get_by_id))
                .route("/{id}", web::put().to(update))
                .route("/{id}", web::delete().to(delete)),
        );
    }

    fn build_test_app() -> App<
        impl actix_web::dev::ServiceFactory<
            actix_web::dev::ServiceRequest,
            Config = (),
            Response = actix_web::dev::ServiceResponse,
            Error = actix_web::Error,
            InitError = (),
        >,
    > {
        App::new()
            .app_data(web::Data::new(JwtSecret(TEST_SECRET.to_string())))
            .app_data(web::Data::new(dummy_pool()))
            .configure(test_app_config)
    }

    #[actix_web::test]
    async fn test_list_users_forbidden_for_non_admin() {
        let app = test::init_service(build_test_app()).await;

        let token = generate_token(1, "regular", "user", 3, TEST_SECRET).unwrap();

        let req = test::TestRequest::get()
            .uri("/api/users")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn test_get_other_user_forbidden_for_non_admin() {
        let app = test::init_service(build_test_app()).await;

        let token = generate_token(1, "regular", "user", 3, TEST_SECRET).unwrap();

        let req = test::TestRequest::get()
            .uri("/api/users/999")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn test_update_other_user_forbidden_for_non_admin() {
        let app = test::init_service(build_test_app()).await;

        let token = generate_token(1, "regular", "user", 3, TEST_SECRET).unwrap();

        let req = test::TestRequest::put()
            .uri("/api/users/999")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "first_name": "Hacker"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn test_update_role_forbidden_for_non_admin() {
        let app = test::init_service(build_test_app()).await;

        let token = generate_token(1, "regular", "user", 3, TEST_SECRET).unwrap();

        let req = test::TestRequest::put()
            .uri("/api/users/1")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "role_id": 1
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn test_update_is_active_forbidden_for_non_admin() {
        let app = test::init_service(build_test_app()).await;

        let token = generate_token(1, "regular", "user", 3, TEST_SECRET).unwrap();

        let req = test::TestRequest::put()
            .uri("/api/users/1")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "is_active": false
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn test_delete_forbidden_for_non_admin() {
        let app = test::init_service(build_test_app()).await;

        let token = generate_token(1, "regular", "user", 3, TEST_SECRET).unwrap();

        let req = test::TestRequest::delete()
            .uri("/api/users/1")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn test_all_user_endpoints_require_auth() {
        let app = test::init_service(build_test_app()).await;

        let endpoints = vec![
            ("GET", "/api/users/me"),
            ("GET", "/api/users"),
            ("GET", "/api/users/1"),
            ("DELETE", "/api/users/1"),
        ];

        for (method, uri) in endpoints {
            let req = match method {
                "GET" => test::TestRequest::get().uri(uri).to_request(),
                "DELETE" => test::TestRequest::delete().uri(uri).to_request(),
                _ => unreachable!(),
            };

            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                StatusCode::UNAUTHORIZED,
                "{method} {uri} should require authentication"
            );
        }

        let req = test::TestRequest::put()
            .uri("/api/users/1")
            .set_json(serde_json::json!({"first_name": "Test"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
