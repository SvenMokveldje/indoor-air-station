mod api;
mod config;
mod db;
mod ingest;
use std::time::Duration;

use axum::Router;
use config::Config;
use db::Database;
use rumqttc::{AsyncClient, MqttOptions};

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

    let mut mqtt_options = MqttOptions::new(&config.mqtt_device_name, &config.mqtt_host, 1883);
    mqtt_options.set_keep_alive(Duration::from_secs(5));

    let (client, eventloop) = AsyncClient::new(mqtt_options, 10);
    let mqtt_client = ingest::MQTTClient::new(client, eventloop, db.clone());
    mqtt_client.spawn();

    let app = init_routers(db);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("failed to bind adress");
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

fn init_routers(db: Database) -> Router {
    let state = AppState { db };

    Router::new().merge(api::reading_routes()).with_state(state)
}
