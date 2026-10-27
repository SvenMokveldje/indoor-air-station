mod config;
mod db;
use config::Config;
use db::Database;

fn main() {
    let config = Config::load("config.toml").unwrap_or_else(|err| {
        eprint!("Configuration error: {err}");
        std::process::exit(1);
    });

    println!("{:?}", config);

    let db = Database::open(config.database_path).unwrap_or_else(|err| {
        eprint!("Database error: {err}");
        std::process::exit(1);
    });

}
