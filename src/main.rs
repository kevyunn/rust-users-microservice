mod config;
mod db;
mod errors;
mod handlers;
mod middleware;
mod models;
mod schema;
mod services;

use actix_web::{App, HttpServer, middleware::Logger, web};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let app_config = config::AppConfig::from_env();
    let pool = db::establish_connection_pool(&app_config.database_url);

    db::run_migrations(&pool);

    let jwt_secret = config::JwtSecret(app_config.jwt_secret.clone());
    let bind_address = format!("{}:{}", app_config.server_host, app_config.server_port);
    log::info!("Starting server at {bind_address}");

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(jwt_secret.clone()))
            .configure(handlers::configure)
    })
    .bind(&bind_address)?
    .run()
    .await
}
