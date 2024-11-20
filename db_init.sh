# Scylladb needs to be running as a separate container
# docker volume create scylla_data
# docker run --name scylla -d -p 9042:9042 -v scylla_data:/var/lib/scylla scylladb/scylla

# Connect to ScyllaDB container
docker exec -it scylla cqlsh

CREATE KEYSPACE IF NOT EXISTS twitter_clone 
WITH replication = {'class': 'NetworkTopologyStrategy', 'datacenter1': 3};

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

# CREATE MATERIALIZED VIEW IF NOT EXISTS twitter_clone.users_by_username AS
# SELECT * FROM twitter_clone.users
# WHERE username IS NOT NULL
# PRIMARY KEY (username, user_id);

# CREATE MATERIALIZED VIEW IF NOT EXISTS twitter_clone.users_by_email AS
# SELECT * FROM twitter_clone.users
# WHERE email IS NOT NULL
# PRIMARY KEY (email, user_id);

CREATE TABLE IF NOT EXISTS twitter_clone.tweets (
    tweet_id uuid,
    user_id uuid,
    content text,
    created_at timestamp,
    PRIMARY KEY ((user_id), tweet_id)
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