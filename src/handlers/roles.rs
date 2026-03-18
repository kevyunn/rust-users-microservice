use actix_web::{web, HttpResponse};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::db::DbPool;
use crate::errors::{AppError, ErrorResponse};
use crate::models::role::Role;
use crate::middleware::auth::AuthenticatedUser;
use crate::services::role as role_service;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRoleRequest {
    pub name: String,
}

#[derive(Debug, serde::Serialize, Deserialize, ToSchema)]
pub struct PermissionItem {
    pub resource: String,
    pub action: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePermissionsRequest {
    pub permissions: Vec<PermissionItem>,
}

#[utoipa::path(
    get,
    path = "/api/roles",
    responses(
        (status = 200, description = "List of roles", body = Vec<Role>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse)
    ),
    security((), ("bearer_auth" = []))
)]
pub async fn list(
    pool: web::Data<DbPool>,
    auth_user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    let pool = pool.into_inner();
    let role_id = auth_user.role_id;

    web::block({
        let pool = pool.clone();
        move || role_service::require_permission(&pool, role_id, "roles", "read")
    })
    .await??;

    let roles = web::block(move || role_service::list_roles(&pool)).await??;
    Ok(HttpResponse::Ok().json(roles))
}

#[utoipa::path(
    post,
    path = "/api/roles",
    request_body = CreateRoleRequest,
    responses(
        (status = 201, description = "Created role", body = Role),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse)
    ),
    security((), ("bearer_auth" = []))
)]
pub async fn create(
    pool: web::Data<DbPool>,
    auth_user: AuthenticatedUser,
    body: web::Json<CreateRoleRequest>,
) -> Result<HttpResponse, AppError> {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Role name is required".into()));
    }
    if name.len() > 50 {
        return Err(AppError::BadRequest(
            "Role name must be at most 50 characters".into(),
        ));
    }
    let pool = pool.into_inner();
    let role_id = auth_user.role_id;

    web::block({
        let pool = pool.clone();
        move || role_service::require_permission(&pool, role_id, "roles", "create")
    })
    .await??;

    let role = web::block(move || role_service::create_role(&pool, name)).await??;
    Ok(HttpResponse::Created().json(role))
}

#[utoipa::path(
    get,
    path = "/api/roles/{id}/permissions",
    params(("id" = i32, Path, description = "Role ID")),
    responses(
        (status = 200, description = "Role permissions", body = Vec<PermissionItem>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse)
    ),
    security((), ("bearer_auth" = []))
)]
pub async fn get_permissions(
    pool: web::Data<DbPool>,
    auth_user: AuthenticatedUser,
    path: web::Path<i32>,
) -> Result<HttpResponse, AppError> {
    let target_role_id = path.into_inner();
    let pool = pool.into_inner();
    let auth_role_id = auth_user.role_id;

    web::block({
        let pool = pool.clone();
        move || role_service::require_permission(&pool, auth_role_id, "roles", "read")
    })
    .await??;

    let permissions =
        web::block(move || role_service::get_role_permissions(&pool, target_role_id)).await??;
    let response: Vec<PermissionItem> = permissions
        .into_iter()
        .map(|p| PermissionItem {
            resource: p.resource,
            action: p.action,
        })
        .collect();
    Ok(HttpResponse::Ok().json(response))
}

#[utoipa::path(
    put,
    path = "/api/roles/{id}/permissions",
    params(("id" = i32, Path, description = "Role ID")),
    request_body = UpdatePermissionsRequest,
    responses(
        (status = 200, description = "Updated permissions", body = Vec<PermissionItem>),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse)
    ),
    security((), ("bearer_auth" = []))
)]
pub async fn update_permissions(
    pool: web::Data<DbPool>,
    auth_user: AuthenticatedUser,
    path: web::Path<i32>,
    body: web::Json<UpdatePermissionsRequest>,
) -> Result<HttpResponse, AppError> {
    let target_role_id = path.into_inner();
    let pool = pool.into_inner();
    let auth_role_id = auth_user.role_id;

    web::block({
        let pool = pool.clone();
        move || role_service::require_permission(&pool, auth_role_id, "roles", "update")
    })
    .await??;

    let items: Vec<(String, String)> = body
        .permissions
        .iter()
        .map(|p| (p.resource.clone(), p.action.clone()))
        .collect();
    let permissions =
        web::block(move || role_service::update_role_permissions(&pool, target_role_id, items))
            .await??;
    let response: Vec<PermissionItem> = permissions
        .into_iter()
        .map(|p| PermissionItem {
            resource: p.resource,
            action: p.action,
        })
        .collect();
    Ok(HttpResponse::Ok().json(response))
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

    const TEST_SECRET: &str = "test-secret-for-roles";

    fn dummy_pool() -> DbPool {
        let manager = ConnectionManager::<PgConnection>::new("postgres://fake@localhost/fake");
        r2d2::Pool::builder()
            .max_size(1)
            .build_unchecked(manager)
    }

    fn test_app_config(cfg: &mut web::ServiceConfig) {
        cfg.service(
            web::scope("/api/roles")
                .route("", web::get().to(list))
                .route("", web::post().to(create))
                .route("/{id}/permissions", web::get().to(get_permissions))
                .route("/{id}/permissions", web::put().to(update_permissions)),
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
    async fn test_list_roles_forbidden_for_non_admin() {
        let app = test::init_service(build_test_app()).await;

        let token = generate_token(1, "regular", "user", 3, TEST_SECRET).unwrap();

        let req = test::TestRequest::get()
            .uri("/api/roles")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn test_create_role_forbidden_for_non_admin() {
        let app = test::init_service(build_test_app()).await;

        let token = generate_token(1, "regular", "user", 3, TEST_SECRET).unwrap();

        let req = test::TestRequest::post()
            .uri("/api/roles")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({ "name": "custom_role" }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn test_get_permissions_forbidden_for_non_admin() {
        let app = test::init_service(build_test_app()).await;

        let token = generate_token(1, "regular", "user", 3, TEST_SECRET).unwrap();

        let req = test::TestRequest::get()
            .uri("/api/roles/1/permissions")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn test_update_permissions_forbidden_for_non_admin() {
        let app = test::init_service(build_test_app()).await;

        let token = generate_token(1, "regular", "user", 3, TEST_SECRET).unwrap();

        let req = test::TestRequest::put()
            .uri("/api/roles/1/permissions")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "permissions": [{ "resource": "users", "action": "read" }]
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn test_all_roles_endpoints_require_auth() {
        let app = test::init_service(build_test_app()).await;

        let endpoints = vec![
            ("GET", "/api/roles"),
            ("GET", "/api/roles/1/permissions"),
        ];

        for (method, uri) in endpoints {
            let req = test::TestRequest::get().uri(uri).to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                StatusCode::UNAUTHORIZED,
                "{method} {uri} should require authentication"
            );
        }

        let req = test::TestRequest::post()
            .uri("/api/roles")
            .set_json(serde_json::json!({ "name": "new_role" }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let req = test::TestRequest::put()
            .uri("/api/roles/1/permissions")
            .set_json(serde_json::json!({
                "permissions": [{ "resource": "users", "action": "read" }]
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
