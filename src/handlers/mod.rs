pub mod auth;
pub mod health;
pub mod roles;
pub mod users;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/health", web::get().to(health::health_check))
            .service(
                web::scope("/auth")
                    .route("/register", web::post().to(auth::register))
                    .route("/login", web::post().to(auth::login)),
            )
            .service(
                web::scope("/users")
                    .route("/me", web::get().to(users::me))
                    .route("", web::get().to(users::list))
                    .route("/{id}", web::get().to(users::get_by_id))
                    .route("/{id}", web::put().to(users::update))
                    .route("/{id}", web::delete().to(users::delete)),
            )
            .service(
                web::scope("/roles")
                    .route("", web::get().to(roles::list))
                    .route("", web::post().to(roles::create))
                    .route("/{id}/permissions", web::get().to(roles::get_permissions))
                    .route("/{id}/permissions", web::put().to(roles::update_permissions)),
            ),
    );
}
