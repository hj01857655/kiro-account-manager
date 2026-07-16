use crate::{error::AppResult, gateway, models::ApiResponse, state::AppState};
use axum::{extract::State, response::Response, Json};
use serde_json::Value;

pub async fn models(State(state): State<AppState>) -> AppResult<Json<ApiResponse<Value>>> {
    let models = gateway::available_models(&state).await?;
    Ok(Json(ApiResponse::ok(models)))
}

pub async fn chat(State(state): State<AppState>, Json(body): Json<Value>) -> AppResult<Response> {
    gateway::user_chat(state, body).await
}
