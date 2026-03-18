use actix_web::{HttpResponse, web};
use diesel::RunQueryDsl;
use utoipa::ToSchema;

use crate::db::DbPool;

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub db: String,
}

#[utoipa::path(
    get,
    path = "/api/health",
    responses(
        (status = 200, description = "Health check", body = HealthResponse)
    )
)]
pub async fn health_check(pool: web::Data<DbPool>) -> HttpResponse {
    let pool = pool.into_inner();

    let db_status = web::block(move || -> Result<String, String> {
        let mut conn = pool.get().map_err(|e| e.to_string())?;
        diesel::sql_query("SELECT 1")
            .execute(&mut conn)
            .map_err(|e| e.to_string())?;
        Ok("connected".to_string())
    })
    .await;

    let db_status = match db_status {
        Ok(Ok(status)) => status,
        Ok(Err(e)) => format!("error: {e}"),
        Err(e) => format!("error: {e}"),
    };

    let status = if db_status == "connected" {
        "ok"
    } else {
        "error"
    };

    HttpResponse::Ok().json(HealthResponse {
        status: status.to_string(),
        db: db_status,
    })
}
