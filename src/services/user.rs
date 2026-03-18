use diesel::prelude::*;

use crate::db::DbPool;
use crate::errors::AppError;
use crate::models::role::Role;
use crate::models::user::{UpdateUser, User, UserResponse};
use crate::schema::{roles, users};

pub fn find_user_by_id(pool: &DbPool, user_id: i32) -> Result<UserResponse, AppError> {
    let mut conn = pool.get().map_err(|e| AppError::DbError(e.to_string()))?;

    let user: User = users::table.find(user_id).first(&mut conn)?;
    let role: Role = roles::table.find(user.role_id).first(&mut conn)?;

    Ok(UserResponse::from_user(user, role.name))
}

pub fn list_users(
    pool: &DbPool,
    page: i64,
    per_page: i64,
) -> Result<(Vec<UserResponse>, i64), AppError> {
    let mut conn = pool.get().map_err(|e| AppError::DbError(e.to_string()))?;

    let total: i64 = users::table.count().get_result(&mut conn)?;

    let offset = (page - 1) * per_page;

    let results: Vec<(User, Role)> = users::table
        .inner_join(roles::table)
        .order(users::id.asc())
        .offset(offset)
        .limit(per_page)
        .select((User::as_select(), Role::as_select()))
        .load(&mut conn)?;

    let user_responses: Vec<UserResponse> = results
        .into_iter()
        .map(|(user, role)| UserResponse::from_user(user, role.name))
        .collect();

    Ok((user_responses, total))
}

pub fn update_user(
    pool: &DbPool,
    user_id: i32,
    email: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    password: Option<String>,
    role_id: Option<i32>,
    is_active: Option<bool>,
    is_admin: bool,
) -> Result<UserResponse, AppError> {
    let mut conn = pool.get().map_err(|e| AppError::DbError(e.to_string()))?;

    let _existing: User = users::table.find(user_id).first(&mut conn)?;

    if let Some(ref pwd) = password {
        if pwd.len() < 8 {
            return Err(AppError::BadRequest(
                "Password must be at least 8 characters".into(),
            ));
        }
        let hash = crate::services::auth::hash_password(pwd)?;
        diesel::update(users::table.find(user_id))
            .set(users::password_hash.eq(hash))
            .execute(&mut conn)?;
    }

    let effective_role_id = if is_admin { role_id } else { None };
    let effective_is_active = if is_admin { is_active } else { None };

    if let Some(rid) = effective_role_id {
        let _role: Role = roles::table
            .find(rid)
            .first(&mut conn)
            .map_err(|_| AppError::BadRequest(format!("Role with id {} not found", rid)))?;
    }

    let changeset = UpdateUser {
        email,
        first_name,
        last_name,
        is_active: effective_is_active,
        role_id: effective_role_id,
        updated_at: Some(chrono::Utc::now().naive_utc()),
    };

    diesel::update(users::table.find(user_id))
        .set(&changeset)
        .execute(&mut conn)?;

    let user: User = users::table.find(user_id).first(&mut conn)?;
    let role: Role = roles::table.find(user.role_id).first(&mut conn)?;

    Ok(UserResponse::from_user(user, role.name))
}

pub fn deactivate_user(pool: &DbPool, user_id: i32) -> Result<(), AppError> {
    let mut conn = pool.get().map_err(|e| AppError::DbError(e.to_string()))?;

    let _existing: User = users::table.find(user_id).first(&mut conn)?;

    diesel::update(users::table.find(user_id))
        .set((
            users::is_active.eq(false),
            users::updated_at.eq(chrono::Utc::now().naive_utc()),
        ))
        .execute(&mut conn)?;

    Ok(())
}
