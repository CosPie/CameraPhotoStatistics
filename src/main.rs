use analyze::*;
use axum::{
    extract::Json,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{path::Path, time::Instant};
pub mod analyze;

#[derive(Serialize, Deserialize)]
pub struct ReportReqParams {
    path: String,
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(serve_index))
        .route("/report", post(handle_report));

    let port = "3000";
    println!("Start at http://127.0.0.1:{}", port);
    axum::Server::bind(&format!("0.0.0.0:{}", port).parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn serve_index() -> impl IntoResponse {
    let index_html = include_str!("../report/index.html");
    Html(index_html)
}

async fn handle_report(
    Json(params): Json<ReportReqParams>,
) -> (StatusCode, axum::response::Json<Value>) {
    let start = Instant::now();

    let output_record = create_output_record();
    let path = params.path;
    println!("path:{}", path);

    let scan_folder_path = Path::new(&path);

    scan_exif_from_folder(scan_folder_path, &output_record);
    let result = get_analyze_result(&output_record).expect("illegal data");

    let duration = start.elapsed();
    println!("Duration: {:?}", duration);

    (StatusCode::OK, Json(json!(result)))
}
