use diesel::prelude::*;
use serde::Serialize;

use crate::schema::permissions;

#[derive(Debug, Queryable, Selectable, Identifiable, Associations, Serialize, Clone)]
#[diesel(table_name = permissions)]
#[diesel(belongs_to(super::role::Role))]
pub struct Permission {
    pub id: i32,
    pub role_id: i32,
    pub resource: String,
    pub action: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = permissions)]
pub struct NewPermission {
    pub role_id: i32,
    pub resource: String,
    pub action: String,
}
