mod config;
mod db;
mod errors;
mod handlers;
mod middleware;
mod models;
mod openapi;
mod schema;
mod services;

use actix_web::{web, App, HttpServer, middleware::Logger};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

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
    log::info!("Swagger UI: http://{bind_address}/swagger-ui/");

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(jwt_secret.clone()))
            .configure(handlers::configure)
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", openapi::ApiDoc::openapi()),
            )
    })
    .bind(&bind_address)?
    .run()
    .await
}
