use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use serde::Serialize;
use std::fmt;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    error: String,
    message: String,
}

#[derive(Debug)]
pub enum AppError {
    InternalError(String),
    DbError(String),
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::InternalError(msg) => write!(f, "Internal error: {msg}"),
            AppError::DbError(msg) => write!(f, "Database error: {msg}"),
            AppError::NotFound(msg) => write!(f, "Not found: {msg}"),
            AppError::BadRequest(msg) => write!(f, "Bad request: {msg}"),
            AppError::Unauthorized(msg) => write!(f, "Unauthorized: {msg}"),
            AppError::Forbidden(msg) => write!(f, "Forbidden: {msg}"),
        }
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::InternalError(_) | AppError::DbError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let error_type = match self {
            AppError::InternalError(_) => "internal_error",
            AppError::DbError(_) => "database_error",
            AppError::NotFound(_) => "not_found",
            AppError::BadRequest(_) => "bad_request",
            AppError::Unauthorized(_) => "unauthorized",
            AppError::Forbidden(_) => "forbidden",
        };

        HttpResponse::build(self.status_code()).json(ErrorResponse {
            error: error_type.to_string(),
            message: self.to_string(),
        })
    }
}

impl From<diesel::result::Error> for AppError {
    fn from(err: diesel::result::Error) -> Self {
        match err {
            diesel::result::Error::NotFound => AppError::NotFound("Record not found".to_string()),
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                info,
            ) => AppError::BadRequest(format!("Already exists: {}", info.message())),
            _ => AppError::DbError(err.to_string()),
        }
    }
}

impl From<actix_web::error::BlockingError> for AppError {
    fn from(_: actix_web::error::BlockingError) -> Self {
        AppError::InternalError("Server error: blocking operation cancelled".to_string())
    }
}
