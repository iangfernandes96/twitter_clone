## Running DB
docker-compose up --build

## Seeding data

cargo run --bin seed

## Running server

cargo run --bin twitter_clone

## Running Load test

# On macOS
brew install k6

k6 run --out json=test_results.json load_test.js