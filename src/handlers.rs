use crate::db::ScyllaConnectionManager;
use bb8::Pool;

use actix_web::{get, post, web, HttpResponse};
use bcrypt::{hash, DEFAULT_COST};
use chrono::{DateTime, Utc};
use log::{debug, error, info};
use scylla::{
    frame::response::result::CqlValue, frame::value::CqlTimestamp, transport::errors::QueryError,
    QueryResult,
};
use uuid::Uuid;
//type DbPool = Arc<Mutex<Vec<Session>>>;
type DbPool = Pool<ScyllaConnectionManager>;

use crate::models::{CreateTweetRequest, CreateUserRequest, Tweet, User};
use scylla::batch::Batch;

#[post("/users")]
pub async fn create_user(
    db_pool: web::Data<DbPool>,
    user_data: web::Json<CreateUserRequest>,
) -> HttpResponse {
    let password_hash = match hash(user_data.password.as_bytes(), DEFAULT_COST) {
        Ok(hash) => hash,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let user_id = Uuid::new_v4();
    let now = Utc::now();
    let cql_timestamp = CqlTimestamp(now.timestamp_millis());

    let session = match db_pool.get().await {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let result = session
            .query(
                "INSERT INTO twitter_clone.users (user_id, username, email, password_hash, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
                (
                    user_id,
                    &user_data.username,
                    &user_data.email,
                    &password_hash,
                    cql_timestamp,
                    cql_timestamp
                ),
            )
            .await;
    match result {
        Ok(_) => {
            let user = User {
                user_id,
                username: user_data.username.clone(),
                email: user_data.email.clone(),
                password_hash,
                created_at: now,
                updated_at: now,
            };
            HttpResponse::Ok().json(user)
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[post("/tweets")]
pub async fn create_tweet(
    db_pool: web::Data<DbPool>,
    tweet_data: web::Json<CreateTweetRequest>,
    query: web::Query<UserIdQuery>,
) -> HttpResponse {
    let tweet_id = Uuid::new_v4();
    let user_id = match Uuid::parse_str(&query.user_id) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    let now = Utc::now();
    let cql_timestamp = CqlTimestamp(now.timestamp_millis());

    let session = match db_pool.get().await {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    // Prepare batch query
    let mut batch: Batch = Default::default();

    batch.append_statement(
        "INSERT INTO twitter_clone.tweets (tweet_id, user_id, content, created_at) VALUES (?, ?, ?, ?)",
    );
    batch.append_statement(
        "INSERT INTO twitter_clone.user_timeline (user_id, tweet_id, created_at) VALUES (?, ?, ?)",
    );

    let batch_values = (
        (tweet_id, user_id, &tweet_data.content, cql_timestamp),
        (user_id, tweet_id, cql_timestamp),
    );

    // Execute batch query
    match session.batch(&batch, batch_values).await {
        Ok(_) => {
            let tweet = Tweet {
                tweet_id,
                user_id,
                content: tweet_data.content.clone(),
                created_at: now,
            };
            HttpResponse::Ok().json(tweet)
        }
        Err(err) => {
            error!("Failed to execute batch query: {:?}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/tweets/{tweet_id}/like")]
pub async fn like_tweet(
    db_pool: web::Data<DbPool>,
    tweet_id: web::Path<String>,
    query: web::Query<UserIdQuery>,
) -> HttpResponse {
    let tweet_id = match Uuid::parse_str(&tweet_id) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    let user_id = match Uuid::parse_str(&query.user_id) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    let now = Utc::now();
    let cql_timestamp = CqlTimestamp(now.timestamp_millis());

    let session = match db_pool.get().await {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let result = session
        .query(
            "INSERT INTO twitter_clone.likes (tweet_id, user_id, created_at) VALUES (?, ?, ?)",
            (tweet_id, user_id, cql_timestamp),
        )
        .await;

    match result {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[get("/feed")]
pub async fn get_home_feed(
    db_pool: web::Data<DbPool>,
    query: web::Query<UserIdQuery>,
) -> HttpResponse {
    let user_id = match Uuid::parse_str(&query.user_id) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let session = match db_pool.get().await {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let result: Result<QueryResult, QueryError> = session
        .query(
            "SELECT tweet_id FROM twitter_clone.user_timeline WHERE user_id = ? LIMIT 20",
            (user_id,),
        )
        .await;

    match result {
        Ok(rows) => {
            let tweet_ids: Vec<Uuid> = rows
                .rows
                .unwrap_or_default()
                .into_iter()
                .filter_map(|row| match row.columns[0].as_ref() {
                    Some(CqlValue::Uuid(uuid)) => Some(*uuid),
                    _ => None,
                })
                .collect();

            let mut tweets = Vec::new();
            for tweet_id in tweet_ids {
                if let Ok(result) = session
                        .query(
                            "SELECT tweet_id, user_id, content, created_at FROM twitter_clone.tweets WHERE tweet_id = ?",
                            (tweet_id,),
                        )
                        .await
                    {
                        if let Some(rows) = result.rows {
                            for row in rows {
                                let tweet = match (
                                    row.columns[0].as_ref().and_then(|v| match v {
                                        CqlValue::Uuid(uuid) => Some(*uuid),
                                        _ => None,
                                    }),
                                    row.columns[1].as_ref().and_then(|v| match v {
                                        CqlValue::Uuid(uuid) => Some(*uuid),
                                        _ => None,
                                    }),
                                    row.columns[2].as_ref().and_then(|v| match v {
                                        CqlValue::Text(text) => Some(text.clone()),
                                        _ => None,
                                    }),
                                    row.columns[3].as_ref().and_then(|v| match v {
                                        CqlValue::Timestamp(ts) => Some(*ts),
                                        _ => None,
                                    }),
                                ) {
                                    (Some(tweet_id), Some(user_id), Some(content), Some(timestamp)) => {
                                        let timestamp_millis = timestamp.0; // Extract the inner i64 value from CqlTimestamp
                                        let seconds = timestamp_millis / 1000;
                                        let nanos = ((timestamp_millis % 1000) * 1_000_000) as u32;
                                        let created_at = DateTime::<Utc>::from_timestamp(
                                            seconds,
                                            nanos
                                        ).unwrap_or_default();
                                        Some(Tweet {
                                            tweet_id,
                                            user_id,
                                            content,
                                            created_at,
                                        })
                                    }
                                    _ => None,
                                };
                                if let Some(tweet) = tweet {
                                    tweets.push(tweet);
                                }
                            }
                        }
                    }
            }
            HttpResponse::Ok().json(tweets)
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[get("/users/{user_id}/tweets")]
pub async fn get_user_tweets(
    db_pool: web::Data<DbPool>,
    user_id: web::Path<String>,
) -> HttpResponse {
    let user_id = match Uuid::parse_str(&user_id) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    info!("Fetching tweets for user: {}", user_id);

    let session = match db_pool.get().await {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let result = session
        .query(
            "SELECT tweet_id, user_id, content, created_at FROM twitter_clone.tweets WHERE user_id = ? ALLOW FILTERING",
            (user_id,),
        )
        .await;
    match result {
        Ok(rows) => {
            let mut tweets = Vec::new();
            if let Some(rows) = rows.rows {
                for row in rows {
                    let tweet = match (
                        row.columns[0].as_ref().and_then(|v| match v {
                            CqlValue::Uuid(uuid) => Some(*uuid),
                            _ => None,
                        }),
                        row.columns[1].as_ref().and_then(|v| match v {
                            CqlValue::Uuid(uuid) => Some(*uuid),
                            _ => None,
                        }),
                        row.columns[2].as_ref().and_then(|v| match v {
                            CqlValue::Text(text) => Some(text.clone()),
                            _ => None,
                        }),
                        row.columns[3].as_ref().and_then(|v| match v {
                            CqlValue::Timestamp(ts) => Some(*ts),
                            _ => None,
                        }),
                    ) {
                        (Some(tweet_id), Some(user_id), Some(content), Some(timestamp)) => {
                            let timestamp_millis = timestamp.0; // Extract the inner i64 value from CqlTimestamp
                            let seconds = timestamp_millis / 1000;
                            let nanos = ((timestamp_millis % 1000) * 1_000_000) as u32;
                            let created_at =
                                DateTime::<Utc>::from_timestamp(seconds, nanos).unwrap_or_default();
                            Some(Tweet {
                                tweet_id,
                                user_id,
                                content,
                                created_at,
                            })
                        }
                        _ => None,
                    };

                    if let Some(tweet) = tweet {
                        tweets.push(tweet);
                    }
                }
            }
            debug!("Found {} tweets for user {}", tweets.len(), user_id);
            HttpResponse::Ok().json(tweets)
        }
        Err(e) => {
            error!("Failed to fetch user tweets: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[derive(serde::Deserialize)]
pub struct UserIdQuery {
    user_id: String,
}
