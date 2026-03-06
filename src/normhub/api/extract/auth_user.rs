use std::sync::Arc;

use crate::{
    api::{self, router::JwtExt},
    core::jwt,
};
use axum::{extract::FromRequestParts, http::request::Parts, Extension, RequestPartsExt};
use axum_extra::TypedHeader;
use headers::{authorization::Bearer, Authorization};

pub struct AuthUser(pub i64);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = api::Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| api::Error::BadRequest("invalid token".to_string()))?;

        let jwt_ext: Extension<Arc<JwtExt>> =
            Extension::from_request_parts(parts, state).await.unwrap();

        let user_id = jwt::user_id(bearer.token(), &jwt_ext.secret)
            .map_err(|_| api::Error::BadRequest("invalid token".to_string()))?;

        Ok(AuthUser(user_id))
    }
}
