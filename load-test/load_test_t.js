import { randomString } from 'https://jslib.k6.io/k6-utils/1.2.0/index.js';
import { check, sleep } from 'k6';
import http from 'k6/http';
import { Rate, Trend } from 'k6/metrics';

// Configuration - Reduced targets for faster completion
const TARGET_USERS = 10_000;  // Reduced from 1M to 10K
const MIN_TWEETS_PER_USER = 5;  // Reduced from 10
const MAX_TWEETS_PER_USER = 20; // Reduced from 1000
const BATCH_SIZE = 100;  // Smaller batch size for more frequent updates

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
        // Faster stages
        { duration: '30s', target: 200 },   // Quick ramp up to 200 VUs
        { duration: '5m', target: 200 },    // Maintain high load for 5 minutes
        { duration: '30s', target: 100 },   // Reduce for mixed operations
        { duration: '5m', target: 100 },    // Mixed operations for 5 minutes
        { duration: '30s', target: 0 },     // Quick ramp down
    ],
    thresholds: {
        http_req_duration: ['p(95)<5000'],
        'create_user_duration': ['p(95)<6000'],
        'create_tweet_duration': ['p(95)<4000'],
        'get_user_tweets_duration': ['p(95)<3000'],
        'get_feed_duration': ['p(95)<3000'],
    },
    // Batch requests for better performance
    batch: 20,  // Send up to 20 requests in parallel
};

// Keep track of created users during the test
let createdUsers = new Set();

function createRandomUser() {
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
        headers: { 'Content-Type': 'application/json' },
        tags: { name: 'CreateUser' },
    };

    const startTime = new Date();
    const res = http.post('http://localhost:8080/api/users', payload, params);
    const duration = new Date() - startTime;
    createUserTrend.add(duration);

    const success = check(res, {
        'user created successfully': (r) => r.status === 200,
    });

    if (!success) {
        createUserErrors.add(1);
        return null;
    }

    createUserErrors.add(0);
    const user = JSON.parse(res.body);
    createdUsers.add(user.user_id);

    if (createdUsers.size % BATCH_SIZE === 0) {
        console.log(`Created ${createdUsers.size}/${TARGET_USERS} users`);
    }

    return user;
}

function createTweetsForUser(userId, count) {
    // Prepare all tweets in a batch
    const requests = [];
    for (let i = 0; i < count; i++) {
        requests.push({
            method: 'POST',
            url: `http://localhost:8080/api/tweets?user_id=${userId}`,
            body: JSON.stringify({
                content: `Tweet ${i} from user ${userId}: ${randomString(20)}`
            }),
            params: {
                headers: { 'Content-Type': 'application/json' },
                tags: { name: 'CreateTweet' },
            }
        });
    }

    // Send tweets in batches of 10
    const TWEET_BATCH_SIZE = 10;
    for (let i = 0; i < requests.length; i += TWEET_BATCH_SIZE) {
        const batch = requests.slice(i, i + TWEET_BATCH_SIZE);
        const startTime = new Date();
        const responses = http.batch(batch);
        const duration = new Date() - startTime;

        responses.forEach(res => {
            createTweetTrend.add(duration / batch.length);
            const success = check(res, {
                'tweet created successfully': (r) => r.status === 200,
            });
            createTweetErrors.add(success ? 0 : 1);
            if (success) totalTweetsCreated++;
        });
    }

    if (totalTweetsCreated % 1000 === 0) {
        console.log(`Total tweets created: ${totalTweetsCreated}`);
    }
}

export default function () {
    // First phase: Create users and their initial tweets
    if (createdUsers.size < TARGET_USERS) {
        const user = createRandomUser();
        if (user) {
            const tweetCount = Math.floor(Math.random() * (MAX_TWEETS_PER_USER - MIN_TWEETS_PER_USER + 1)) + MIN_TWEETS_PER_USER;
            createTweetsForUser(user.user_id, tweetCount);
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
            createTweetsForUser(userId, 2); // Reduced tweet count
        } else if (action < 0.7) {
            // Get user tweets (30%)
            const startTime = new Date();
            const res = http.get(`http://localhost:8080/api/users/${userId}/tweets`);
            getUserTweetsTrend.add(new Date() - startTime);
            getUserTweetsErrors.add(check(res, { 'success': (r) => r.status === 200 }) ? 0 : 1);
        } else {
            // Get feed (30%)
            const startTime = new Date();
            const res = http.get(`http://localhost:8080/api/feed?user_id=${userId}`);
            getFeedTrend.add(new Date() - startTime);
            getFeedErrors.add(check(res, { 'success': (r) => r.status === 200 }) ? 0 : 1);
        }
    }

    sleep(0.1); // Reduced sleep time
}

export function handleSummary(data) {
    console.log(`Test completed:
    - Users created: ${createdUsers.size}
    - Tweets created: ${totalTweetsCreated}
    - Error rates:
      * Users: ${data.metrics.create_user_errors?.rate || 0}
      * Tweets: ${data.metrics.create_tweet_errors?.rate || 0}
    `);

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