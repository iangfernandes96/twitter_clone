# Scylladb needs to be running as a separate container
# docker run --name scylla -d -p 9042:9042 scylladb/scylla

# Connect to ScyllaDB container
docker exec -it scylla cqlsh

CREATE KEYSPACE IF NOT EXISTS twitter_clone 
WITH replication = {'class': 'SimpleStrategy', 'replication_factor': 1};

CREATE TABLE IF NOT EXISTS twitter_clone.users (
    user_id uuid PRIMARY KEY,
    username text,
    email text,
    password_hash text,
    created_at timestamp,
    updated_at timestamp
);

CREATE INDEX IF NOT EXISTS users_username_idx ON twitter_clone.users (username);
CREATE INDEX IF NOT EXISTS users_email_idx ON twitter_clone.users (email);

CREATE TABLE IF NOT EXISTS twitter_clone.tweets (
    tweet_id uuid,
    user_id uuid,
    content text,
    created_at timestamp,
    PRIMARY KEY (tweet_id)
);

CREATE TABLE IF NOT EXISTS twitter_clone.user_timeline (
    user_id uuid,
    tweet_id uuid,
    created_at timestamp,
    PRIMARY KEY (user_id, created_at, tweet_id)
) WITH CLUSTERING ORDER BY (created_at DESC);

CREATE TABLE IF NOT EXISTS twitter_clone.likes (
    tweet_id uuid,
    user_id uuid,
    created_at timestamp,
    PRIMARY KEY (tweet_id, user_id)
);