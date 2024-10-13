use std::str::FromStr;

use actix_cors::Cors;
use actix_web::{
    middleware::Logger,
    web::{self, Data},
    App, HttpServer,
};
use actix_web_httpauth::middleware::HttpAuthentication;
use chrono::Local;
use config::Config;
use controllers::{auth_controller, file_controller, user_controller};
use cron::Schedule;
use dotenv::dotenv;
use middleware::validator;
use services::db::Database;
use tokio::time;

mod config;
mod controllers;
mod dtos;
mod middleware;
mod models;
mod services;
mod utils;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok(); // Load the .env file
    let config = Config::init();
    let config_data = Data::new(config);
    let db = Database::init(config_data.database_url.clone().to_string()).await;
    let db_data = Data::new(db);
    let port = config_data.port.clone().to_string();
    let db_data_for_cron = db_data.clone();
    tokio::spawn(async move {
        start_cron_jobs(db_data_for_cron).await;
    });
    HttpServer::new(move || {
        let logger = Logger::default();
        let auth = HttpAuthentication::bearer(validator);
        // Configure CORS
        let cors = Cors::default()
            .allow_any_origin() // or use .allowed_origin("http://example.com")
            .allow_any_method()
            .allow_any_header()
            .supports_credentials(); // Optional if you need credentials
        App::new()
            .wrap(logger)
            .wrap(cors)
            .app_data(db_data.clone())
            .app_data(config_data.clone())
            .configure(auth_controller::init)
            .service(
                web::scope("/user")
                    .wrap(auth.clone())
                    .configure(user_controller::init),
            )
            .service(
                web::scope("/file")
                    .wrap(auth.clone())
                    .configure(file_controller::init),
            )
    })
    .bind(format!(":{}", port))?
    .run()
    .await
}

async fn start_cron_jobs(db_client: Data<Database>) {
    // Schedule a cron job to run every day at midnight
    let schedule = Schedule::from_str("0 0 * * * *").unwrap();
    let mut next = schedule.upcoming(Local);

    loop {
        if let Some(chrono_time) = next.next() {
            let now = Local::now();
            let duration = (chrono_time - now).to_std().unwrap();

            // Wait until the next scheduled time
            time::sleep(duration).await;

            // Run the job
            println!("Running scheduled task to delete expired files...");
            if let Err(err) = db_client.delete_expired_files().await {
                eprintln!("Error deleting expired files: {:?}", err);
            } else {
                println!("Successfully deleted expired files.");
            }
            next = schedule.upcoming(Local); // Update the next schedule
        }
    }
}
