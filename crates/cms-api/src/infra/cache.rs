use fred::{
    clients::RedisPool,
    interfaces::KeysInterface,
    types::{Expiration, Scanner},
};
use futures::StreamExt;
use serde::{Serialize, de::DeserializeOwned};

pub async fn get<T>(redis: &RedisPool, key: &str) -> Option<T>
where
    T: DeserializeOwned,
{
    match redis.get::<Option<String>, _>(key).await {
        Ok(Some(raw)) => match serde_json::from_str(&raw) {
            Ok(value) => Some(value),
            Err(error) => {
                tracing::warn!(key, %error, "cache decode failed");
                None
            }
        },
        Ok(None) => None,
        Err(error) => {
            tracing::warn!(key, %error, "cache get failed");
            None
        }
    }
}

pub async fn set<T>(redis: &RedisPool, key: &str, value: &T, ttl_secs: i64)
where
    T: Serialize,
{
    let raw = match serde_json::to_string(value) {
        Ok(raw) => raw,
        Err(error) => {
            tracing::warn!(key, %error, "cache encode failed");
            return;
        }
    };

    if let Err(error) = redis
        .set::<(), _, _>(key, raw, Some(Expiration::EX(ttl_secs)), None, false)
        .await
    {
        tracing::warn!(key, %error, "cache set failed");
    }
}

pub async fn del_one(redis: &RedisPool, key: &str) {
    if let Err(error) = redis.del::<i64, _>(key).await {
        tracing::warn!(key, %error, "cache delete failed");
    }
}

pub async fn invalidate_prefix(redis: &RedisPool, pattern: &str) {
    let mut keys = Vec::new();
    let mut stream = redis.next().scan(pattern.to_owned(), Some(100), None);

    while let Some(page) = stream.next().await {
        match page {
            Ok(mut scan) => {
                if let Some(page_keys) = scan.take_results() {
                    keys.extend(page_keys.into_iter().filter_map(|key| key.into_string()));
                }
            }
            Err(error) => {
                tracing::warn!(pattern, %error, "cache scan failed");
                return;
            }
        }
    }

    if keys.is_empty() {
        return;
    }

    if let Err(error) = redis.unlink::<i64, _>(keys).await {
        tracing::warn!(pattern, %error, "cache unlink failed");
    }
}
