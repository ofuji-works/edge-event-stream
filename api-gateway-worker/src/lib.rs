use shared::AppError;
use tower_service::Service;

// auth
//
// ingest
//
// R2  algo
//
//     d1

#[derive(derive_new::new, Clone)]
struct AppState {
    pub ctx: std::sync::Arc<worker::Context>,
    pub env: std::sync::Arc<worker::Env>,
}

fn router(state: AppState) -> axum::Router {
    axum::Router::new()
        .route("/", axum::routing::post(post))
        .with_state(state)
}

#[worker::event(fetch)]
async fn fetch(
    req: worker::HttpRequest,
    env: worker::Env,
    ctx: worker::Context,
) -> worker::Result<axum::http::Response<axum::body::Body>> {
    let state = AppState::new(std::sync::Arc::new(ctx), std::sync::Arc::new(env));
    Ok(router(state).call(req).await?)
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Post {
    id: String,
}

#[axum::debug_handler]
async fn post(
    axum::extract::State(_state): axum::extract::State<AppState>,
    axum::Json(json): axum::Json<Post>,
) -> Result<axum::Json<Post>, AppError> {
    Ok(axum::Json(json))
}
