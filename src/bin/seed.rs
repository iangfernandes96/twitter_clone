use bcrypt::{hash, DEFAULT_COST};
use chrono::Utc;
use fake::faker::internet::en::{SafeEmail, Username};
use fake::faker::lorem::en::Sentence;
use fake::Fake;
use scylla::{frame::value::CqlTimestamp, Session, SessionBuilder};
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn Error>> {
//     println!("Starting data seeding...");

//     // Connect to ScyllaDB
//     let session = SessionBuilder::new()
//         .known_node("127.0.0.1:9042")
//         .build()
//         .await?;

//     // Configuration
//     let num_users = 10000000;
//     let tweets_per_user = 600;

//     let users = seed_users(&session, num_users).await?;
//     seed_tweets(&session, &users, tweets_per_user).await?;

//     println!("Seeding completed!");
//     Ok(())
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Starting data seeding...");

    // Connect to ScyllaDB
    let session = Arc::new(
        SessionBuilder::new()
            .known_node("127.0.0.1:9042")
            .build()
            .await?,
    );

    // Configuration
    let num_users = 1_000_000;
    let tweets_per_user = 100;

    let users = seed_users(session.clone(), num_users).await?;
    seed_tweets(session.clone(), &users, tweets_per_user).await?;

    println!("Seeding completed!");
    Ok(())
}
// async fn seed_users(session: &Session, count: i32) -> Result<Vec<Uuid>, Box<dyn Error>> {
//     println!("Creating {} users...", count);
//     let mut users = Vec::new();

//     for i in 0..count {
//         let user_id = Uuid::new_v4();
//         let username: String = Username().fake();
//         let email: String = SafeEmail().fake();
//         let password_hash = hash("password123", DEFAULT_COST)?;
//         let now = CqlTimestamp(Utc::now().timestamp_millis());

//         session
//             .query(
//                 "INSERT INTO twitter_clone.users (user_id, username, email, password_hash, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
//                 (user_id, &username, &email, &password_hash, now, now),
//             )
//             .await?;

//         users.push(user_id);
//         println!(
//             "Created user {}/{}: {} ({})",
//             i + 1,
//             count,
//             username,
//             user_id
//         );
//     }

//     Ok(users)
// }

async fn seed_users(
    session: Arc<Session>,
    count: i32,
) -> Result<Vec<Uuid>, Box<dyn Error + Send + Sync>> {
    println!("Creating {} users...", count);

    // Configuration
    let chunk_size = 10_000; // Number of users per task
    let num_chunks = (count as usize + chunk_size - 1) / chunk_size; // Round up to ensure all users are processed
    let mut tasks = Vec::new();
    let mut users = Vec::new();

    for chunk_id in 0..num_chunks {
        let session_clone = session.clone();
        let start = chunk_id * chunk_size;
        let end = ((chunk_id + 1) * chunk_size).min(count as usize);

        tasks.push(tokio::spawn(async move {
            let mut chunk_users = Vec::new();
            for i in start..end {
                let user_id = Uuid::new_v4();
                let username: String = Username().fake();
                let email: String = SafeEmail().fake();
                let password_hash = hash("password123", DEFAULT_COST)?;
                let now = CqlTimestamp(Utc::now().timestamp_millis());

                session_clone
                    .query(
                        "INSERT INTO twitter_clone.users (user_id, username, email, password_hash, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
                        (user_id, &username, &email, &password_hash, now, now),
                    )
                    .await?;

                chunk_users.push(user_id);

                if (i - start + 1) % 1_000 == 0 {
                    println!(
                        "Chunk {}/{}: Created {}/{} users",
                        chunk_id + 1,
                        num_chunks,
                        i - start + 1,
                        end - start
                    );
                }
            }
            Ok::<Vec<Uuid>, Box<dyn Error + Send + Sync>>(chunk_users)
        }));
    }

    // Collect results
    for task in tasks {
        users.extend(task.await??);
    }

    println!("Created a total of {} users.", users.len());
    Ok(users)
}

// async fn seed_tweets(
//     session: Arc<Session>,
//     users: &[Uuid],
//     tweets_per_user: i32,
// ) -> Result<(), Box<dyn Error>> {
//     println!("Creating {} tweets per user...", tweets_per_user);
//     let total_tweets = users.len() as i32 * tweets_per_user;
//     let mut current_tweet = 0;

//     for &user_id in users {
//         for _ in 0..tweets_per_user {
//             let tweet_id = Uuid::new_v4();
//             let content: String = Sentence(3..10).fake();
//             let now = CqlTimestamp(Utc::now().timestamp_millis());

//             // Insert tweet
//             session
//                 .query(
//                     "INSERT INTO twitter_clone.tweets (tweet_id, user_id, content, created_at) VALUES (?, ?, ?, ?)",
//                     (tweet_id, user_id, &content, now),
//                 )
//                 .await?;

//             // Insert into user timeline
//             session
//                 .query(
//                     "INSERT INTO twitter_clone.user_timeline (user_id, tweet_id, created_at) VALUES (?, ?, ?)",
//                     (user_id, tweet_id, now),
//                 )
//                 .await?;

//             current_tweet += 1;
//             if current_tweet % 100 == 0 {
//                 println!("Created {}/{} tweets", current_tweet, total_tweets);
//             }
//         }
//     }

//     Ok(())
// }

async fn seed_tweets(
    session: Arc<Session>,
    users: &[Uuid],
    tweets_per_user: i32,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Creating {} tweets per user...", tweets_per_user);

    // Configuration
    let chunk_size = 10; // Number of users per task
    let total_tweets = users.len() as i32 * tweets_per_user;
    let mut current_tweet = 0;
    let mut tasks = Vec::new();

    for chunk in users.chunks(chunk_size) {
        let session_clone = session.clone();
        let chunk_users = chunk.to_vec();
        tasks.push(tokio::spawn(async move {
            for &user_id in &chunk_users {
                for _ in 0..tweets_per_user {
                    let tweet_id = Uuid::new_v4();
                    let content: String = Sentence(3..10).fake();
                    let now = CqlTimestamp(Utc::now().timestamp_millis());

                    // Insert tweet
                    session_clone
                        .query(
                            "INSERT INTO twitter_clone.tweets (tweet_id, user_id, content, created_at) VALUES (?, ?, ?, ?)",
                            (tweet_id, user_id, &content, now),
                        )
                        .await?;

                    // Insert into user timeline
                    session_clone
                        .query(
                            "INSERT INTO twitter_clone.user_timeline (user_id, tweet_id, created_at) VALUES (?, ?, ?)",
                            (user_id, tweet_id, now),
                        )
                        .await?;
                }
            }
            Ok::<(), Box<dyn Error + Send + Sync>>(())
        }));
    }

    // Wait for all tasks
    for task in tasks {
        task.await??;
        current_tweet += chunk_size * tweets_per_user as usize;
        println!("Created {}/{} tweets", current_tweet, total_tweets);
    }

    Ok(())
}
