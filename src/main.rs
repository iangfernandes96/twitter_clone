mod db;
mod handlers;
mod models;

use actix_web::{web, App, HttpServer};
use env_logger::Builder;

use log::{info, LevelFilter};
use std::sync::Arc;

#[actix_web::main]
// async fn main() -> std::io::Result<()> {
//     Builder::new()
//         .filter_level(LevelFilter::Debug)
//         .format_timestamp_secs()
//         .init();

//     info!("Starting Twitter clone backend...");
//     // Connect to ScyllaDB
//     let session = db::create_session()
//         .await
//         .expect("Failed to create database session");
//     let session = Arc::new(session);
//     let db_pool = db::create_connection_pool()
//         .await
//         .expect("Failed to created db pool");
//     info!("Connected to ScyllaDB");

//     // let num_workers = num_cpus::get().max(4);
//     // info!(
//     //     "Starting Twitter clone backend with {} workers...",
//     //     num_workers
//     // );
//     HttpServer::new(move || {
//         App::new()
//             .app_data(web::Data::new(db_pool.clone()))
//             .service(
//                 web::scope("/api")
//                     .service(handlers::create_user)
//                     .service(handlers::create_tweet)
//                     .service(handlers::like_tweet)
//                     .service(handlers::get_home_feed)
//                     .service(handlers::get_user_tweets),
//             )
//     })
//     .bind("127.0.0.1:8080")?
//     .run()
//     .await
// }

async fn main() -> std::io::Result<()> {
    Builder::new()
        .filter_level(LevelFilter::Debug)
        .format_timestamp_secs()
        .init();

    info!("Starting Twitter clone backend...");
    // Connect to ScyllaDB
    let session = db::create_session()
        .await
        .expect("Failed to create database session");
    let session = Arc::new(session);
    let db_pool = db::create_connection_pool()
        .await
        .expect("Failed to created db pool");
    info!("Connected to ScyllaDB");

    let num_workers = num_cpus::get().max(4);
    info!(
        "Starting Twitter clone backend with {} workers...",
        num_workers
    );

    // Start HTTP server
    // HttpServer::new(move || {
    //     App::new()
    //         .app_data(web::Data::new(session.clone()))
    //         .service(
    //             web::scope("/api")
    //                 .service(handlers::create_user)
    //                 .service(handlers::create_tweet)
    //                 .service(handlers::like_tweet)
    //                 .service(handlers::get_home_feed)
    //                 .service(handlers::get_user_tweets),
    //         )
    // })
    // .bind("127.0.0.1:8080")?
    // .run()
    // .await
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db_pool.clone()))
            .service(
                web::scope("/api")
                    .service(handlers::create_user)
                    .service(handlers::create_tweet)
                    .service(handlers::like_tweet)
                    .service(handlers::get_home_feed)
                    .service(handlers::get_user_tweets),
            )
    })
    .workers(num_workers)
    .max_connections(1024)
    .max_connection_rate(256)
    .backlog(1024)
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
