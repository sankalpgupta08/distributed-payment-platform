use std::time::Duration;

use redis::{Client, aio::ConnectionManager};
use uuid::Uuid;

const RELEASE_IF_OWNED: &str = r#"
if redis.call('GET', KEYS[1]) == ARGV[1] then
    return redis.call('DEL', KEYS[1])
end
return 0
"#;

/// A Redis-backed, ownership-safe lease lock.
#[derive(Clone)]
pub struct RedisLock {
    connection: ConnectionManager,
    ttl: Duration,
}

pub struct RedisLockGuard {
    lock: RedisLock,
    key: String,
    token: String,
}

impl RedisLock {
    /// Connects to Redis and verifies that it is responsive at startup.
    pub async fn connect(redis_url: &str, ttl: Duration) -> redis::RedisResult<Self> {
        let client = Client::open(redis_url)?;
        let connection = ConnectionManager::new(client).await?;
        let lock = Self { connection, ttl };
        lock.ping().await?;
        Ok(lock)
    }

    /// Acquires a lease only if no current owner holds `key`.
    pub async fn acquire(&self, key: String) -> redis::RedisResult<Option<RedisLockGuard>> {
        let token = Uuid::new_v4().to_string();
        let ttl_millis = self.ttl.as_millis().try_into().unwrap_or(u64::MAX);
        let mut connection = self.connection.clone();
        let result: Option<String> = redis::cmd("SET")
            .arg(&key)
            .arg(&token)
            .arg("NX")
            .arg("PX")
            .arg(ttl_millis)
            .query_async(&mut connection)
            .await?;

        Ok(result.map(|_| RedisLockGuard {
            lock: self.clone(),
            key,
            token,
        }))
    }

    pub async fn ping(&self) -> redis::RedisResult<()> {
        let mut connection = self.connection.clone();
        let _: String = redis::cmd("PING").query_async(&mut connection).await?;
        Ok(())
    }
}

impl RedisLockGuard {
    /// Releases the lease only when this guard still owns it.
    pub async fn release(self) -> redis::RedisResult<()> {
        let mut connection = self.lock.connection.clone();
        let _: i64 = redis::cmd("EVAL")
            .arg(RELEASE_IF_OWNED)
            .arg(1)
            .arg(&self.key)
            .arg(&self.token)
            .query_async(&mut connection)
            .await?;
        Ok(())
    }
}
