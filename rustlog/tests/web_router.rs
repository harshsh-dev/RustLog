use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tokio::sync::broadcast;
use tower::ServiceExt;

use rustlog::web_dashboard;

#[tokio::test]
async fn dashboard_root_returns_html() {
    let (tx, _) = broadcast::channel(8);
    let app = web_dashboard::router(tx);
    let res = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let s = String::from_utf8_lossy(&body);
    assert!(s.contains("WebSocket"), "body={s:?}");
}
