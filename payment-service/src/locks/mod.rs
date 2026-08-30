//! Distributed coordination primitives backed by Redis.

mod redis_lock;

pub use redis_lock::RedisLock;
