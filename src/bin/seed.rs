use bcrypt::{hash, DEFAULT_COST};
use chrono::Utc;
use fake::faker::internet::en::{SafeEmail, Username};
use fake::faker::lorem::en::Sentence;
use fake::{Fake, Faker};
use scylla::{frame::value::CqlTimestamp, Session, SessionBuilder};
use std::error::Error;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting data seeding...");

    // Connect to ScyllaDB
    let session = SessionBuilder::new()
        .known_node("127.0.0.1:9042")
        .build()
        .await?;

    // Configuration
    let num_users = 100;
    let tweets_per_user = 20;

    let users = seed_users(&session, num_users).await?;
    seed_tweets(&session, &users, tweets_per_user).await?;

    println!("Seeding completed!");
    Ok(())
}

async fn seed_users(session: &Session, count: i32) -> Result<Vec<Uuid>, Box<dyn Error>> {
    println!("Creating {} users...", count);
    let mut users = Vec::new();

    for i in 0..count {
        let user_id = Uuid::new_v4();
        let username: String = Username().fake();
        let email: String = SafeEmail().fake();
        let password_hash = hash("password123", DEFAULT_COST)?;
        let now = CqlTimestamp(Utc::now().timestamp_millis());

        session
            .query(
                "INSERT INTO twitter_clone.users (user_id, username, email, password_hash, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
                (user_id, &username, &email, &password_hash, now, now),
            )
            .await?;

        users.push(user_id);
        println!(
            "Created user {}/{}: {} ({})",
            i + 1,
            count,
            username,
            user_id
        );
    }

    Ok(users)
}

async fn seed_tweets(
    session: &Session,
    users: &[Uuid],
    tweets_per_user: i32,
) -> Result<(), Box<dyn Error>> {
    println!("Creating {} tweets per user...", tweets_per_user);
    let total_tweets = users.len() as i32 * tweets_per_user;
    let mut current_tweet = 0;

    for &user_id in users {
        for _ in 0..tweets_per_user {
            let tweet_id = Uuid::new_v4();
            let content: String = Sentence(3..10).fake();
            let now = CqlTimestamp(Utc::now().timestamp_millis());

            // Insert tweet
            session
                .query(
                    "INSERT INTO twitter_clone.tweets (tweet_id, user_id, content, created_at) VALUES (?, ?, ?, ?)",
                    (tweet_id, user_id, &content, now),
                )
                .await?;

            // Insert into user timeline
            session
                .query(
                    "INSERT INTO twitter_clone.user_timeline (user_id, tweet_id, created_at) VALUES (?, ?, ?)",
                    (user_id, tweet_id, now),
                )
                .await?;

            current_tweet += 1;
            if current_tweet % 100 == 0 {
                println!("Created {}/{} tweets", current_tweet, total_tweets);
            }
        }
    }

    Ok(())
}
