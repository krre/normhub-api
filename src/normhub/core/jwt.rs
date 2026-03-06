use std::{
    ops::Add,
    time::{Duration, SystemTime},
};

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    user_id: i64,
    exp: usize,
}

#[derive(Error, Debug)]
#[error("jsonwebtoken error: {0}")]
pub struct Error(#[from] jsonwebtoken::errors::Error);

pub fn create_token(user_id: i64, email: &str, secret: &str) -> Result<String, Error> {
    let from_now = Duration::from_secs(3600 * 24 * 365 * 10); // 10 years
    let expired_future_time = SystemTime::now().add(from_now);
    let exp = OffsetDateTime::from(expired_future_time);

    let claims = Claims {
        sub: String::from(email),
        exp: exp.unix_timestamp() as usize,
        user_id,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok(token)
}

pub fn user_id(token: &str, secret: &str) -> Result<i64, Error> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )?;

    Ok(data.claims.user_id)
}
