mod config;
mod db;
mod errors;
mod handlers;
mod models;
mod schema;

use actix_web::{App, HttpServer, middleware::Logger, web};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let config = config::AppConfig::from_env();
    let pool = db::establish_connection_pool(&config.database_url);

    db::run_migrations(&pool);

    let bind_address = format!("{}:{}", config.server_host, config.server_port);
    log::info!("Starting server at {bind_address}");

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(pool.clone()))
            .configure(handlers::configure)
    })
    .bind(&bind_address)?
    .run()
    .await
}
