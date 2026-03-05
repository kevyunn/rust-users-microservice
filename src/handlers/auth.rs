use actix_web::{web, HttpResponse};
use diesel::prelude::*;
use serde::Deserialize;

use crate::config::JwtSecret;
use crate::db::DbPool;
use crate::errors::AppError;
use crate::middleware::auth::AuthenticatedUser;
use crate::models::role::Role;
use crate::models::user::{NewUser, User, UserResponse};
use crate::schema::{roles, users};
use crate::services::auth;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub first_name: String,
    pub last_name: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, serde::Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

pub async fn register(
    pool: web::Data<DbPool>,
    body: web::Json<RegisterRequest>,
) -> Result<HttpResponse, AppError> {
    let body = body.into_inner();

    if body.username.trim().is_empty() || body.email.trim().is_empty() || body.password.is_empty() {
        return Err(AppError::BadRequest(
            "Username, email and password are required".into(),
        ));
    }

    if body.password.len() < 8 {
        return Err(AppError::BadRequest(
            "Password must be at least 8 characters".into(),
        ));
    }

    let pool = pool.into_inner();

    let user_response = web::block(move || {
        let mut conn = pool.get().map_err(|e| AppError::DbError(e.to_string()))?;

        let password_hash = auth::hash_password(&body.password)?;

        let role: Role = roles::table
            .filter(roles::name.eq("user"))
            .first(&mut conn)
            .map_err(|_| AppError::InternalError("Default role 'user' not found".into()))?;

        let new_user = NewUser {
            username: body.username,
            email: body.email,
            password_hash,
            first_name: body.first_name,
            last_name: body.last_name,
            role_id: role.id,
        };

        let user: User = diesel::insert_into(users::table)
            .values(&new_user)
            .get_result(&mut conn)?;

        Ok::<UserResponse, AppError>(UserResponse::from_user(user, role.name))
    })
    .await??;

    Ok(HttpResponse::Created().json(user_response))
}

pub async fn login(
    pool: web::Data<DbPool>,
    jwt_secret: web::Data<JwtSecret>,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse, AppError> {
    let body = body.into_inner();
    let secret = jwt_secret.into_inner();
    let pool = pool.into_inner();

    let login_response = web::block(move || {
        let mut conn = pool.get().map_err(|e| AppError::DbError(e.to_string()))?;

        let user: User = users::table
            .filter(users::username.eq(&body.username))
            .first(&mut conn)
            .map_err(|_| AppError::Unauthorized("Invalid username or password".into()))?;

        if !user.is_active {
            return Err(AppError::Unauthorized("Account is deactivated".into()));
        }

        let is_valid = auth::verify_password(&body.password, &user.password_hash)?;
        if !is_valid {
            return Err(AppError::Unauthorized(
                "Invalid username or password".into(),
            ));
        }

        let role: Role = roles::table.find(user.role_id).first(&mut conn)?;

        let token = auth::generate_token(user.id, &user.username, &role.name, &secret.0)?;

        Ok::<LoginResponse, AppError>(LoginResponse {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_in: 24 * 3600,
        })
    })
    .await??;

    Ok(HttpResponse::Ok().json(login_response))
}

pub async fn me(
    pool: web::Data<DbPool>,
    auth_user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    let pool = pool.into_inner();

    let user_response = web::block(move || {
        let mut conn = pool.get().map_err(|e| AppError::DbError(e.to_string()))?;

        let user: User = users::table.find(auth_user.user_id).first(&mut conn)?;

        let role: Role = roles::table.find(user.role_id).first(&mut conn)?;

        Ok::<UserResponse, AppError>(UserResponse::from_user(user, role.name))
    })
    .await??;

    Ok(HttpResponse::Ok().json(user_response))
}
