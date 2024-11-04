import { randomString } from 'https://jslib.k6.io/k6-utils/1.2.0/index.js';
import { check, sleep } from 'k6';
import http from 'k6/http';
import { Rate, Trend } from 'k6/metrics';

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

export const options = {
    stages: [
        { duration: '1s', target: 5 },   // Ramp up to 5 users
        { duration: '10s', target: 5 },    // Stay at 5 users
        { duration: '2s', target: 10 },  // Ramp up to 10 users
        { duration: '10s', target: 10 },   // Stay at 10 users
        { duration: '3s', target: 0 },   // Ramp down
    ],
    thresholds: {
        // Response time thresholds
        http_req_duration: ['p(95)<2000'], // 95% of requests should be below 2s
        'create_user_duration': ['p(95)<3000'],
        'create_tweet_duration': ['p(95)<2000'],
        'get_user_tweets_duration': ['p(95)<1000'],
        'get_feed_duration': ['p(95)<1000'],

        // Error rate thresholds
        'create_user_errors': ['rate<0.1'],    // Less than 10% error rate
        'create_tweet_errors': ['rate<0.1'],
        'get_user_tweets_errors': ['rate<0.1'],
        'get_feed_errors': ['rate<0.1'],
    },
};

// Keep track of created users during the test
let createdUsers = new Set();

function createRandomUser() {
    const username = `user_${randomString(8)}`;
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
    return user;
}

function createTweet(userId) {
    const payload = JSON.stringify({
        content: `Test tweet ${randomString(20)}`
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
        return null;
    }

    createTweetErrors.add(0);
    return JSON.parse(res.body);
}

export default function () {
    // Ensure we have at least one user
    if (createdUsers.size === 0) {
        const user = createRandomUser();
        if (!user) return;
    }

    // Random action
    const action = Math.random();

    // Select a random user
    const userIds = Array.from(createdUsers);
    const userId = userIds[Math.floor(Math.random() * userIds.length)];

    if (action < 0.2) {
        // Create new user and tweets (20%)
        const user = createRandomUser();
        if (user) {
            const numTweets = Math.floor(Math.random() * 3) + 1;
            for (let i = 0; i < numTweets; i++) {
                createTweet(user.user_id);
            }
        }
    } else if (action < 0.5) {
        // Create tweet (30%)
        createTweet(userId);
    } else if (action < 0.75) {
        // Get user tweets (25%)
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
        // Get feed (25%)
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

    // Random sleep between 1-2 seconds
    sleep(1 + Math.random());
}

export function handleSummary(data) {
    return {
        'stdout': JSON.stringify({
            metrics: {
                users_created: createdUsers.size,
                http_req_duration: data.metrics.http_req_duration,
                create_user_errors: data.metrics.create_user_errors,
                create_tweet_errors: data.metrics.create_tweet_errors,
                get_user_tweets_errors: data.metrics.get_user_tweets_errors,
                get_feed_errors: data.metrics.get_feed_errors,
            }
        }, null, 2),
    };
}