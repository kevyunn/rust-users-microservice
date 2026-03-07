use diesel::prelude::*;

use crate::db::DbPool;
use crate::errors::AppError;
use crate::models::permission::Permission;
use crate::models::role::{NewRole, Role};
use crate::schema::{permissions, roles};

pub fn list_roles(pool: &DbPool) -> Result<Vec<Role>, AppError> {
    let mut conn = pool.get().map_err(|e| AppError::DbError(e.to_string()))?;
    roles::table
        .order(roles::id.asc())
        .load::<Role>(&mut conn)
        .map_err(Into::into)
}

pub fn create_role(pool: &DbPool, name: String) -> Result<Role, AppError> {
    if name.len() > 50 {
        return Err(AppError::BadRequest(
            "Role name must be at most 50 characters".into(),
        ));
    }
    let mut conn = pool.get().map_err(|e| AppError::DbError(e.to_string()))?;
    let new_role = NewRole { name };
    diesel::insert_into(roles::table)
        .values(&new_role)
        .get_result::<Role>(&mut conn)
        .map_err(Into::into)
}

pub fn get_role_permissions(pool: &DbPool, role_id: i32) -> Result<Vec<Permission>, AppError> {
    let mut conn = pool.get().map_err(|e| AppError::DbError(e.to_string()))?;
    let _role: Role = roles::table
        .find(role_id)
        .first(&mut conn)
        .map_err(|_| AppError::NotFound(format!("Role with id {} not found", role_id)))?;
    permissions::table
        .filter(permissions::role_id.eq(role_id))
        .order((permissions::resource.asc(), permissions::action.asc()))
        .load::<Permission>(&mut conn)
        .map_err(Into::into)
}

pub fn update_role_permissions(
    pool: &DbPool,
    role_id: i32,
    items: Vec<(String, String)>,
) -> Result<Vec<Permission>, AppError> {
    let mut conn = pool.get().map_err(|e| AppError::DbError(e.to_string()))?;

    let _role: Role = roles::table
        .find(role_id)
        .first(&mut conn)
        .map_err(|_| AppError::NotFound(format!("Role with id {} not found", role_id)))?;

    // Deduplicate by (resource, action) to avoid UNIQUE violation
    let mut seen = std::collections::HashSet::new();
    let unique_items: Vec<_> = items
        .into_iter()
        .filter(|(r, a)| seen.insert((r.clone(), a.clone())))
        .collect();

    for (resource, action) in &unique_items {
        if resource.len() > 50 || action.len() > 50 {
            return Err(AppError::BadRequest(
                "resource and action must be at most 50 characters".into(),
            ));
        }
        if resource.trim().is_empty() || action.trim().is_empty() {
            return Err(AppError::BadRequest(
                "resource and action must be non-empty".into(),
            ));
        }
    }

    conn.transaction::<_, AppError, _>(|conn| {
        diesel::delete(permissions::table.filter(permissions::role_id.eq(role_id)))
            .execute(conn)?;

        for (resource, action) in &unique_items {
            diesel::insert_into(permissions::table)
                .values((
                    permissions::role_id.eq(role_id),
                    permissions::resource.eq(resource),
                    permissions::action.eq(action),
                ))
                .execute(conn)?;
        }

        permissions::table
            .filter(permissions::role_id.eq(role_id))
            .order((permissions::resource.asc(), permissions::action.asc()))
            .load::<Permission>(conn)
            .map_err(Into::into)
    })
}
