use axum::{Json, Router, extract::{Path, State}, http::StatusCode, response::{IntoResponse, Response}, routing::get};
use serde_json::json;
use shared::Reading;

use crate::db::{DBError, Database};

#[derive(Clone)]
pub struct AppState {
    pub db: Database
}

#[derive(Debug)]
pub enum ApiError {
    NotFound,
    BadRequest(String),
    Internal(String),
}

impl From<DBError> for ApiError {
    fn from(err: DBError) -> Self {
        ApiError::Internal(err.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
       let (status, message) = match self {
           ApiError::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
           ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
           ApiError::Internal(details) => {
               eprintln!("internal error: {details}");
               (StatusCode::INTERNAL_SERVER_ERROR, "internal server error".to_string())
           }
       }; 
       (status, Json(json!({"error": message}))).into_response()
    }
}

async fn get_reading(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> Result<Json<Reading>, ApiError> {
    let reading = state.db.get_reading(id).await?.ok_or(ApiError::NotFound)?;
    Ok(Json(reading))
}

pub fn reading_routes() -> Router<AppState> {
    Router::new()
        .route("/readings/{id}", get(get_reading))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::{Body, to_bytes}, http::Request};
use tower::ServiceExt;
    
    async fn body_json(response: Response) -> serde_json::Value {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn new_database() -> Database {
        Database::open_from_memory().expect("in memory db")
    }

    fn test_app(db: Database) -> Router {
        let state = AppState { db };
        reading_routes().with_state(state)
    }

    fn get(uri: &str) -> Request<Body> {
        Request::builder().uri(uri).body(Body::empty()).unwrap()
    }

    async fn insert_reading(db: &Database) -> Result<u32, DBError> {
        let id = 1;
        let reading = Reading {
            seq: id,
            temperature_c: 25.0,
            humidity_pct: 50.0,
            pressure_hpa: 1013.0,
            co2_ppm: Some(400),
        };
    
        db.insert_reading(&reading).await.unwrap();
        Ok(id)
    }

    #[tokio::test]
    async fn not_found_maps_to_404() {
        let response = ApiError::NotFound.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(body_json(response).await["error"], "not found")
    }

    #[tokio::test]
    async fn bad_request_maps_to_400() {
        let response = ApiError::BadRequest("bad request".to_string()).into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body_json(response).await["error"], "bad request")
    }

    #[tokio::test]
    async fn internal_maps_to_500_and_hides_details() {
        let response = ApiError::Internal("table secret_stuff missing".into()).into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let json = body_json(response).await;
        let body = json.to_string();
        assert!(!body.contains("secret_stuff"), "internal details leaked: {body}");
        assert_eq!(json["error"], "internal server error")
    }

    #[tokio::test]
    async fn existing_reading_returns_200_and_json() {
        let db = new_database();
        let id = insert_reading(&db).await.unwrap();
        let app = test_app(db);
        
        let response = app.oneshot(get(&format!("/readings/{id}"))).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body_json(response).await["seq"], id)
    }
}