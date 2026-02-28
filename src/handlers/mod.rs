pub mod health;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/api").route("/health", web::get().to(health::health_check)));
}
