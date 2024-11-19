# Twitter Clone API

A Rust-based Twitter clone API using Actix-web and ScyllaDB, designed to demonstrate high-performance social media backend capabilities.

## Features

- User Management (create users)
- Tweet Operations (create, fetch)
- Timeline Management (user timeline, home feed)
- Three-node ScyllaDB cluster for high availability
- Data seeding capabilities
- Load testing suite
- Grafana + Prometheus Monitoring

## Prerequisites

- Rust (latest stable version)
- Docker and Docker Compose
- k6 (for load testing)

## Getting Started

### 1. Setting up the Database

The project uses a three-node ScyllaDB cluster. Start it using Docker Compose:

```bash
docker-compose up -d --build
```
Wait for all nodes to be ready (usually takes 1-2 minutes). Verify the cluster status:

```bash
docker exec -it scylla-node1 nodetool status
```

### 2. Creating the Schema

Connect to ScyllaDB and create the schema:

```bash
docker exec -it scylla-node1 cqlsh
```
Then paste the CQL commands in db_init.sh


### 3. Running the Application

Build and run the application:

```bash
cargo build --bin twitter_clone
cargo run --bin twitter_clone
```


The API will be available at `http://localhost:8080`

### 4. Seeding Test Data

To populate the database with test data:

```bash
cargo run --bin seed
```


This will create:
- 100 random users
- 20 tweets per user
- Appropriate timeline entries

### 5. Load Testing

Install k6:

```bash
brew install k6
```

### 6. Monitoring

After running 

```bash
docker-compose up --build
```

Navigate to http://localhost:3000

## API Endpoints

### Users
- `POST /api/users` - Create a new user
  ```bash
  curl -X POST http://localhost:8080/api/users \
    -H "Content-Type: application/json" \
    -d '{"username": "testuser", "email": "test@example.com", "password": "password123"}'
  ```

### Tweets
- `POST /api/tweets` - Create a new tweet
  ```bash
  curl -X POST "http://localhost:8080/api/tweets?user_id=USER_ID" \
    -H "Content-Type: application/json" \
    -d '{"content": "Hello, World!"}'
  ```

- `GET /api/users/{user_id}/tweets` - Get user's tweets
  ```bash
  curl "http://localhost:8080/api/users/USER_ID/tweets"
  ```

### Feed
- `GET /api/feed` - Get user's home feed
  ```bash
  curl "http://localhost:8080/api/feed?user_id=USER_ID"
  ```


## Development

### Useful Commands

Check ScyllaDB logs:
```bash
docker-compose logs -f scylla-node1
```

Connect to specific node:
```bash
docker exec -it scylla-node1 cqlsh
```


View cluster status:
```bash
docker exec -it scylla-node1 nodetool status
```


docker-compose up --build

## Seeding data

cargo run --bin seed

## Running server

cargo run --bin twitter_clone

## Running Load test

# On macOS
brew install k6

k6 run --out json=test_results.json load_test.js