use crate::api::Result;
use axum::extract::Path;
use axum::{Json, extract::State};
use sqlx::PgPool;

use crate::api::extract::ValidPayload;

pub(crate) mod router {
    use super::*;
    use axum::routing;
    use sqlx::{Pool, Postgres};

    pub fn new(pool: &Pool<Postgres>) -> routing::Router<Pool<Postgres>> {
        routing::Router::new()
            .route("/", routing::get(get_all))
            .route("/{id}", routing::get(get_one))
            .route("/", routing::post(create))
            .route("/{id}", routing::put(update))
            .route("/{id}", routing::delete(delete))
            .with_state(pool.clone())
    }
}

mod request {
    use serde::Deserialize;
    use validator::Validate;

    #[derive(Deserialize, Validate)]
    pub struct Create {
        pub module_id: Option<i64>,
    }

    #[derive(Deserialize, Validate)]
    pub struct Update {
        pub module_id: Option<i64>,
        #[validate(length(min = 1))]
        pub name: String,
        pub visibility: i16,
    }
}

mod response {
    use serde::Serialize;
    use time::OffsetDateTime;

    #[derive(Serialize)]
    pub struct Create {
        pub id: i64,
        pub name: String,
        pub visibility: i16,
    }

    #[derive(Serialize)]
    pub struct Module {
        pub id: i64,
        pub project_id: i64,
        pub module_id: Option<i64>,
        pub name: String,
        pub visibility: i16,
        pub updated_at: OffsetDateTime,
    }
}

pub async fn create(
    Path(project_id): Path<i64>,
    State(pool): State<PgPool>,
    ValidPayload(payload): ValidPayload<request::Create>,
) -> Result<Json<response::Create>> {
    struct Module {
        id: i64,
    }

    let name =
        "Module.".to_owned() + &next_module_suffix(project_id, payload.module_id, &pool).await?;

    let visibility = 0;

    let module = sqlx::query_as!(
        Module,
        "INSERT INTO modules (project_id, module_id, name, visibility) values ($1, $2, $3, $4) RETURNING id",
        project_id,
        payload.module_id,
        name,
        visibility,
    )
    .fetch_one(&pool)
    .await?;

    Ok(Json(response::Create {
        id: module.id,
        name,
        visibility,
    }))
}

pub async fn update(
    Path(id): Path<i64>,
    State(pool): State<PgPool>,
    ValidPayload(payload): ValidPayload<request::Update>,
) -> Result<()> {
    sqlx::query!(
        "UPDATE modules SET module_id = $1, name = $2, visibility = $3, updated_at = current_timestamp WHERE id = $4",
        payload.module_id,
        payload.name,
        payload.visibility,
        id,
    )
    .execute(&pool)
    .await?;

    Ok(())
}

pub async fn get_all(
    Path(project_id): Path<i64>,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<response::Module>>> {
    let projects = sqlx::query_as!(
        response::Module,
        "SELECT id, project_id, module_id, name, visibility, updated_at
        FROM modules
        WHERE project_id = $1
        ORDER BY updated_at DESC",
        project_id,
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(projects))
}

pub async fn get_one(
    Path(id): Path<i64>,
    State(pool): State<PgPool>,
) -> Result<Json<response::Module>> {
    let project = sqlx::query_as!(
        response::Module,
        "SELECT id, project_id, module_id, name, visibility, updated_at
        FROM modules
        WHERE id = $1",
        id,
    )
    .fetch_one(&pool)
    .await?;

    Ok(Json(project))
}

pub async fn delete(Path(id): Path<i64>, State(pool): State<PgPool>) -> Result<()> {
    sqlx::query!("DELETE FROM modules WHERE id = $1", id)
        .execute(&pool)
        .await?;

    Ok(())
}

async fn next_module_suffix(
    project_id: i64,
    module_id: Option<i64>,
    pool: &PgPool,
) -> Result<String> {
    #[derive(Debug, sqlx::FromRow)]
    struct Module {
        name: String,
    }

    let mut query = sqlx::query_builder::QueryBuilder::new("SELECT name FROM modules WHERE ");

    query.push("project_id = ");
    query.push_bind(project_id);

    if let Some(module_id) = module_id {
        query.push(" AND module_id = ");
        query.push_bind(module_id);
    } else {
        query.push(" AND module_id IS NULL");
    };

    query.push(" ORDER BY name ASC");

    let modules = query.build_query_as::<Module>().fetch_all(pool).await?;

    let mut prev_num = 0;

    for module in modules.iter() {
        let mut name = module.name.chars();
        let dot = name.nth(module.name.len() - 4);

        if let Some(dot) = dot {
            if dot != '.' {
                continue;
            };
        } else {
            continue;
        }

        let num = name.as_str().parse::<u32>();

        if let Ok(num) = num {
            if num - prev_num > 1 {
                break;
            }

            prev_num = num;
        }
    }

    Ok(format!("{:0>3}", prev_num + 1))
}
