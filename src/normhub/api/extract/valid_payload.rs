use axum::{body::Body, extract::FromRequest, http::Request, Json};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::api;

#[derive(Debug, Clone, Copy, Default)]
pub struct ValidPayload<T>(pub T);

impl<T, S> FromRequest<S> for ValidPayload<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = api::Error;

    async fn from_request(req: Request<Body>, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await?;
        value.validate()?;
        Ok(ValidPayload(value))
    }
}
