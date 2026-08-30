use std::{collections::HashSet, time::Duration};

use payment_service::{
    locks::RedisLock,
    models::requests::CreatePaymentRequest,
    services::payment_service,
};
use rust_decimal::Decimal;
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

/// Requires the Docker PostgreSQL and Redis services from docker-compose.yml.
#[tokio::test]
#[ignore = "requires local PostgreSQL and Redis"]
async fn concurrent_identical_requests_create_one_payment() {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");
    let pool: PgPool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("test database should connect");
    let locks = RedisLock::connect(&redis_url, Duration::from_secs(30))
        .await
        .expect("test Redis should connect");
    let idempotency_key = format!("concurrency-test-{}", Uuid::new_v4());
    let merchant_id = Uuid::new_v4();

    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..10 {
        let pool = pool.clone();
        let locks = locks.clone();
        let idempotency_key = idempotency_key.clone();
        tasks.spawn(async move {
            payment_service::create_payment(
                &pool,
                &locks,
                Duration::from_secs(60),
                &idempotency_key,
                CreatePaymentRequest {
                    merchant_id,
                    amount: Decimal::new(50000, 2),
                    currency: "INR".to_owned(),
                },
            )
            .await
        });
    }

    let mut payment_ids = HashSet::new();
    while let Some(result) = tasks.join_next().await {
        let response = result
            .expect("task should not panic")
            .expect("same-key request should succeed or replay");
        assert_eq!(response.status_code, 201);
        payment_ids.insert(response.payment.id);
    }

    assert_eq!(payment_ids.len(), 1, "all callers must receive one payment");

    let database_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM idempotency_keys WHERE key = $1 AND response_body IS NOT NULL",
    )
    .bind(&idempotency_key)
    .fetch_one(&pool)
    .await
    .expect("idempotency record should be queryable");
    assert_eq!(database_count, 1);
}
