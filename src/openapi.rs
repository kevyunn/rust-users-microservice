use utoipa::OpenApi;

use crate::errors::ErrorResponse;
use crate::handlers::auth::{LoginRequest, LoginResponse, RegisterRequest};
use crate::handlers::health::HealthResponse;
use crate::handlers::roles::{CreateRoleRequest, PermissionItem, UpdatePermissionsRequest};
use crate::handlers::users::{PaginatedUsersResponse, UpdateUserRequest};
use crate::models::role::Role;
use crate::models::user::UserResponse;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Users Microservice API",
        version = "0.1.0",
        description = "REST API for user management with roles and RBAC"
    ),
    paths(
        crate::handlers::health::health_check,
        crate::handlers::auth::register,
        crate::handlers::auth::login,
        crate::handlers::users::me,
        crate::handlers::users::list,
        crate::handlers::users::get_by_id,
        crate::handlers::users::update,
        crate::handlers::users::delete,
        crate::handlers::roles::list,
        crate::handlers::roles::create,
        crate::handlers::roles::get_permissions,
        crate::handlers::roles::update_permissions,
    ),
    components(
        schemas(
            HealthResponse,
            RegisterRequest,
            LoginRequest,
            LoginResponse,
            UserResponse,
            UpdateUserRequest,
            PaginatedUsersResponse,
            Role,
            CreateRoleRequest,
            PermissionItem,
            UpdatePermissionsRequest,
            ErrorResponse,
        )
    ),
    tags(
        (name = "health", description = "Health check"),
        (name = "auth", description = "Authentication"),
        (name = "users", description = "User management"),
        (name = "roles", description = "Role management")
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::HttpBuilder::new()
                        .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}
