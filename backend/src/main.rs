mod config;
mod db;
mod api;
use axum::Router;
use config::Config;
use db::Database;

use crate::api::AppState;

#[tokio::main]
async fn main() {
    let config = Config::load("config.toml").unwrap_or_else(|err| {
        eprintln!("Configuration error: {err}");
        std::process::exit(1);
    });

    println!("{:?}", config);

    let db = Database::open(&config.database_path).unwrap_or_else(|err| {
        eprintln!("Database error: {err}");
        std::process::exit(1);
    });

    let app = init_routers(db);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.expect("failed to bind adress");
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

fn init_routers(db: Database) -> Router {
    let state = AppState {
        db,
    };
    
    Router::new()
        .merge(api::reading_routes())
        .with_state(state)
}
