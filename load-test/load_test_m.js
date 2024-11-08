import { randomString } from 'https://jslib.k6.io/k6-utils/1.2.0/index.js';
import { check, sleep } from 'k6';
import http from 'k6/http';
import { Rate, Trend } from 'k6/metrics';

// Configuration
const TARGET_USERS = 1_000_000;
const MIN_TWEETS_PER_USER = 10;
const MAX_TWEETS_PER_USER = 1000;
const BATCH_SIZE = 1000;

// Custom metrics
const createUserErrors = new Rate('create_user_errors');
const createTweetErrors = new Rate('create_tweet_errors');
const getUserTweetsErrors = new Rate('get_user_tweets_errors');
const getFeedErrors = new Rate('get_feed_errors');

// Response time trends
const createUserTrend = new Trend('create_user_duration');
const createTweetTrend = new Trend('create_tweet_duration');
const getUserTweetsTrend = new Trend('get_user_tweets_duration');
const getFeedTrend = new Trend('get_feed_duration');

// Progress tracking
let totalTweetsCreated = 0;

export const options = {
    stages: [
        // User Creation Phase (12 hours)
        { duration: '1h', target: 100 },    // Ramp up to 100 VUs
        { duration: '10h', target: 100 },   // Maintain 100 VUs for user creation
        { duration: '1h', target: 50 },     // Scale down for mixed operations

        // Mixed Operations Phase (12 hours)
        { duration: '1h', target: 50 },     // Maintain 50 VUs
        { duration: '10h', target: 50 },    // Run mixed operations
        { duration: '1h', target: 0 },      // Ramp down
    ],
    thresholds: {
        http_req_duration: ['p(95)<5000'],  // 5s for 95% of requests
        'create_user_duration': ['p(95)<6000'],
        'create_tweet_duration': ['p(95)<4000'],
        'get_user_tweets_duration': ['p(95)<3000'],
        'get_feed_duration': ['p(95)<3000'],
        'create_user_errors': ['rate<0.1'],
        'create_tweet_errors': ['rate<0.1'],
        'get_user_tweets_errors': ['rate<0.1'],
        'get_feed_errors': ['rate<0.1'],
    },
};

// Keep track of created users during the test
let createdUsers = new Set();

function createRandomUser() {
    // Skip if we've reached the target
    if (createdUsers.size >= TARGET_USERS) {
        return null;
    }

    const username = `user_${randomString(8)}_${Date.now()}`;
    const email = `${username}@example.com`;

    const payload = JSON.stringify({
        username: username,
        email: email,
        password: 'password123'
    });

    const params = {
        headers: {
            'Content-Type': 'application/json',
        },
        tags: { name: 'CreateUser' },
    };

    const startTime = new Date();
    const res = http.post('http://localhost:8080/api/users', payload, params);
    const duration = new Date() - startTime;
    createUserTrend.add(duration);

    const success = check(res, {
        'user created successfully': (r) => r.status === 200,
        'valid JSON response': (r) => r.status === 200 && JSON.parse(r.body),
    });

    if (!success) {
        createUserErrors.add(1);
        console.log(`Failed to create user: ${res.status} ${res.body}`);
        return null;
    }

    createUserErrors.add(0);
    const user = JSON.parse(res.body);
    createdUsers.add(user.user_id);

    if (createdUsers.size % BATCH_SIZE === 0) {
        console.log(`Created ${createdUsers.size} users`);
    }

    return user;
}

function createTweetsForUser(userId, count) {
    for (let i = 0; i < count; i++) {
        const payload = JSON.stringify({
            content: `Tweet ${i} from user ${userId}: ${randomString(50)}`
        });

        const params = {
            headers: {
                'Content-Type': 'application/json',
            },
            tags: { name: 'CreateTweet' },
        };

        const startTime = new Date();
        const res = http.post(
            `http://localhost:8080/api/tweets?user_id=${userId}`,
            payload,
            params
        );
        const duration = new Date() - startTime;
        createTweetTrend.add(duration);

        const success = check(res, {
            'tweet created successfully': (r) => r.status === 200,
            'valid JSON response': (r) => r.status === 200 && JSON.parse(r.body),
        });

        if (!success) {
            createTweetErrors.add(1);
            console.log(`Failed to create tweet: ${res.status} ${res.body}`);
            continue;
        }

        createTweetErrors.add(0);
        totalTweetsCreated++;

        if (totalTweetsCreated % 1000 === 0) {
            console.log(`Total tweets created: ${totalTweetsCreated}`);
        }

        // Small sleep to prevent overwhelming the system
        sleep(0.1);
    }
}

export default function () {
    // First phase: Create users and their initial tweets
    if (createdUsers.size < TARGET_USERS) {
        const user = createRandomUser();
        if (user) {
            const tweetCount = Math.floor(Math.random() * (MAX_TWEETS_PER_USER - MIN_TWEETS_PER_USER + 1)) + MIN_TWEETS_PER_USER;
            createTweetsForUser(user.user_id, tweetCount);
            sleep(1); // Sleep between user creation
            return;
        }
    }

    // Second phase: Mixed operations on existing users
    if (createdUsers.size > 0) {
        const userIds = Array.from(createdUsers);
        const userId = userIds[Math.floor(Math.random() * userIds.length)];
        const action = Math.random();

        if (action < 0.4) {
            // Create more tweets (40%)
            createTweetsForUser(userId, Math.floor(Math.random() * 5) + 1);
        } else if (action < 0.7) {
            // Get user tweets (30%)
            const startTime = new Date();
            const res = http.get(`http://localhost:8080/api/users/${userId}/tweets`);
            const duration = new Date() - startTime;
            getUserTweetsTrend.add(duration);

            const success = check(res, {
                'get tweets status is 200': (r) => r.status === 200,
                'valid JSON response': (r) => r.status === 200 && JSON.parse(r.body),
            });
            getUserTweetsErrors.add(success ? 0 : 1);
        } else {
            // Get feed (30%)
            const startTime = new Date();
            const res = http.get(`http://localhost:8080/api/feed?user_id=${userId}`);
            const duration = new Date() - startTime;
            getFeedTrend.add(duration);

            const success = check(res, {
                'get feed status is 200': (r) => r.status === 200,
                'valid JSON response': (r) => r.status === 200 && JSON.parse(r.body),
            });
            getFeedErrors.add(success ? 0 : 1);
        }
    }

    sleep(Math.random() * 2 + 1); // 1-3 second sleep between operations
}

export function handleSummary(data) {
    return {
        'stdout': JSON.stringify({
            metrics: {
                users_created: createdUsers.size,
                tweets_created: totalTweetsCreated,
                http_req_duration: data.metrics.http_req_duration,
                create_user_errors: data.metrics.create_user_errors,
                create_tweet_errors: data.metrics.create_tweet_errors,
                get_user_tweets_errors: data.metrics.get_user_tweets_errors,
                get_feed_errors: data.metrics.get_feed_errors,
            }
        }, null, 2),
    };
}