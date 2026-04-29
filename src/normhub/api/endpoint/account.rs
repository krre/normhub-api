use crate::{
    api::{
        self, Error, Result,
        endpoint::JwtExt,
        extract::{AuthUser, ValidPayload},
    },
    core::jwt,
};
use axum::{Extension, Json, extract::State};
use sqlx::PgPool;
use std::sync::Arc;

pub(crate) mod router {
    use super::*;
    use axum::routing;
    use sqlx::{Pool, Postgres};

    pub fn new(pool: &Pool<Postgres>) -> routing::Router {
        routing::Router::new()
            .route("/", routing::post(create))
            .route("/", routing::get(get_one))
            .route("/", routing::delete(delete))
            .route("/", routing::put(update))
            .route("/login", routing::post(login))
            .route("/password", routing::put(change_password))
            .with_state(pool.clone())
    }
}

mod request {
    use serde::Deserialize;
    use validator::Validate;

    #[derive(Deserialize, Validate)]
    pub struct User {
        #[validate(length(min = 1))]
        pub login: String,
        #[validate(length(min = 1))]
        pub full_name: String,
        #[validate(email)]
        pub email: String,
        #[validate(length(min = 1))]
        pub password: String,
    }

    #[derive(Deserialize, Validate)]
    pub struct UserFullName {
        #[validate(length(min = 1))]
        pub full_name: String,
    }

    #[derive(Deserialize, Validate)]
    pub struct Password {
        #[validate(length(min = 1))]
        pub old_password: String,
        #[validate(length(min = 1))]
        pub new_password: String,
    }

    #[derive(Deserialize, Validate)]
    pub struct LoginUser {
        #[validate(email)]
        pub email: String,
        #[validate(length(min = 1))]
        pub password: String,
    }
}

mod response {
    use serde::Serialize;

    #[derive(Serialize)]
    pub struct Token {
        pub token: String,
    }

    #[derive(Serialize)]
    pub struct Account {
        pub login: String,
        pub full_name: String,
        pub email: String,
    }
}

pub async fn create(
    State(pool): State<PgPool>,
    jwt_ext: Extension<Arc<JwtExt>>,
    ValidPayload(payload): ValidPayload<request::User>,
) -> Result<Json<response::Token>> {
    struct User {
        id: i64,
    }

    let user = sqlx::query_as!(
        User,
        "INSERT INTO users (login, full_name, email, password) values ($1, $2, $3, $4) RETURNING id",
        payload.login,
        payload.full_name,
        payload.email,
        payload.password
    )
    .fetch_one(&pool)
    .await;

    match user {
        Ok(user) => {
            let token = jwt::create_token(user.id as i64, &payload.email, &jwt_ext.secret)
                .map_err(|e| Error::InternalServerError(format!("cannot create token: {}", e)))?;
            return Ok(Json(response::Token { token }));
        }
        Err(error) => match error {
            sqlx::Error::Database(database_error) => {
                if database_error.is_unique_violation() {
                    return Err(Error::Conflict);
                }
            }
            _ => return Err(api::error::Error::DatabaseError(error)),
        },
    }

    Err(Error::InternalServerError("Unexpected error".to_string()))
}

pub async fn update(
    State(pool): State<PgPool>,
    AuthUser(user_id): AuthUser,
    ValidPayload(payload): ValidPayload<request::UserFullName>,
) -> Result<()> {
    sqlx::query!(
        "UPDATE users SET full_name = $1, updated_at = current_timestamp WHERE id = $2",
        payload.full_name,
        user_id
    )
    .execute(&pool)
    .await?;

    Ok(())
}

pub async fn change_password(
    State(pool): State<PgPool>,
    AuthUser(user_id): AuthUser,
    ValidPayload(payload): ValidPayload<request::Password>,
) -> Result<()> {
    struct User {
        password: String,
    }

    let user = sqlx::query_as!(User, "SELECT password FROM users WHERE id = $1", user_id)
        .fetch_one(&pool)
        .await?;

    if user.password != payload.old_password {
        return Err(Error::BadRequest("invalid password".to_string()));
    }

    sqlx::query!(
        "UPDATE users SET password = $1, updated_at = current_timestamp WHERE id = $2",
        payload.new_password,
        user_id
    )
    .execute(&pool)
    .await?;

    Ok(())
}

pub async fn login(
    State(pool): State<PgPool>,
    jwt_ext: Extension<Arc<JwtExt>>,
    payload: axum::extract::Json<request::LoginUser>,
) -> Result<Json<response::Token>> {
    struct User {
        id: i64,
        password: String,
    }

    let user = sqlx::query_as!(
        User,
        "SELECT id, password FROM users WHERE email = $1",
        payload.email,
    )
    .fetch_one(&pool)
    .await;

    match user {
        Ok(user) => {
            if user.password != payload.password {
                return Err(Error::Unauthorized("wrong password".to_string()));
            }

            let token = jwt::create_token(user.id as i64, &payload.email, &jwt_ext.secret)
                .map_err(|e| Error::InternalServerError(format!("cannot create token: {}", e)))?;

            return Ok(Json(response::Token { token }));
        }
        Err(error) => match error {
            sqlx::Error::RowNotFound => {
                return Err(Error::NotFound(format!("email `{}` not found", error)));
            }
            _ => {
                return Err(api::error::Error::DatabaseError(error));
            }
        },
    }
}

pub async fn get_one(
    State(pool): State<PgPool>,
    AuthUser(user_id): AuthUser,
) -> Result<Json<response::Account>> {
    let user = sqlx::query_as!(
        response::Account,
        "SELECT login, full_name, email FROM users WHERE id = $1",
        user_id,
    )
    .fetch_one(&pool)
    .await?;

    Ok(Json(user))
}

pub async fn delete(State(pool): State<PgPool>, AuthUser(user_id): AuthUser) -> Result<()> {
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id,)
        .execute(&pool)
        .await?;

    Ok(())
}
