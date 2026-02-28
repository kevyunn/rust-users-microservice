use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::roles;

#[derive(Debug, Queryable, Selectable, Identifiable, Serialize, Clone)]
#[diesel(table_name = roles)]
pub struct Role {
    pub id: i32,
    pub name: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Insertable, Deserialize)]
#[diesel(table_name = roles)]
pub struct NewRole {
    pub name: String,
}
