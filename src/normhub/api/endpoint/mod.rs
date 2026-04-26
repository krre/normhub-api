pub mod account;
pub mod module;
pub mod project;

use axum::{Extension, middleware, routing::IntoMakeService};
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

use super::middleware::console::log_body;

pub struct Router {
    axum_router: axum::Router,
}

pub struct JwtExt {
    pub secret: String,
}

impl Router {
    pub fn new(pool: Pool<Postgres>, jwt_secret: &str) -> Self {
        let jwt_ext = Arc::new(JwtExt {
            secret: jwt_secret.to_owned(),
        });

        let router = axum::Router::new()
            .nest("/account", account::router::new(&pool))
            .nest("/projects", project::router::new(&pool))
            .layer(TraceLayer::new_for_http())
            .layer(Extension(jwt_ext))
            .layer(middleware::from_fn(log_body));

        Self {
            axum_router: router,
        }
    }

    pub fn into_make_service(self) -> IntoMakeService<axum::Router> {
        self.axum_router.into_make_service()
    }
}
