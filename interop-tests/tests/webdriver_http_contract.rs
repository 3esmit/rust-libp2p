use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{Context, Result};
use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    Json, Router,
};
use serde_json::{json, Value};
use thirtyfour::prelude::{By, DesiredCapabilities, WebDriver, WebDriverError};
use tokio::sync::oneshot;

#[derive(Clone, Default)]
struct RequestLog(Arc<Mutex<Vec<(String, Value)>>>);

async fn webdriver_stub(State(log): State<RequestLog>, request: Request<Body>) -> Response {
    let method = request.method().to_string();
    let path = request.uri().path().to_owned();
    let body = to_bytes(request.into_body(), 1024 * 1024)
        .await
        .context("read WebDriver request body")
        .and_then(|bytes| {
            if bytes.is_empty() {
                Ok(Value::Null)
            } else {
                serde_json::from_slice(&bytes).context("decode WebDriver request JSON")
            }
        });

    let body = match body {
        Ok(body) => body,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"value": {"error": "invalid argument", "message": error.to_string()}})),
            )
                .into_response();
        }
    };
    if let Ok(mut entries) = log.0.lock() {
        entries.push((format!("{method} {path}"), body));
    } else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                json!({"value": {"error": "unknown error", "message": "request log unavailable"}}),
            ),
        )
            .into_response();
    }

    match (method.as_str(), path.as_str()) {
        ("POST", "/session") => (
            StatusCode::OK,
            Json(json!({
                "value": {"sessionId": "session-contract", "capabilities": {}}
            })),
        )
            .into_response(),
        ("POST", "/session/session-contract/timeouts") => {
            (StatusCode::OK, Json(json!({"value": null}))).into_response()
        }
        ("POST", "/session/session-contract/url") => {
            (StatusCode::OK, Json(json!({"value": null}))).into_response()
        }
        ("GET", "/session/session-contract/url") => (
            StatusCode::OK,
            Json(json!({"value": "https://example.test/quoted/%E2%9C%93"})),
        )
            .into_response(),
        ("POST", "/session/session-contract/element") => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "value": {"error": "no such element", "message": "missing element"}
            })),
        )
            .into_response(),
        ("DELETE", "/session/session-contract") => {
            (StatusCode::OK, Json(json!({"value": null}))).into_response()
        }
        _ => (
            StatusCode::NOT_FOUND,
            Json(json!({"value": {"error": "unknown command", "message": "unexpected route"}})),
        )
            .into_response(),
    }
}

#[tokio::test]
async fn webdriver_reqwest_contract_uses_json_and_preserves_response_text() -> Result<()> {
    let log = RequestLog::default();
    let app = Router::new()
        .fallback(webdriver_stub)
        .with_state(log.clone());
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;
    let address = listener.local_addr()?;
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .context("serve WebDriver stub")
    });

    let result = tokio::time::timeout(Duration::from_secs(10), async {
        let driver =
            WebDriver::new(format!("http://{address}"), DesiredCapabilities::chrome()).await?;

        driver.goto("https://example.test/posted/✓").await?;
        let current_url = driver.current_url().await?;
        assert_eq!(
            current_url.as_str(),
            "https://example.test/quoted/%E2%9C%93"
        );

        let Err(error) = driver.find(By::Id("missing")).await else {
            anyhow::bail!("stub should report a missing element")
        };
        assert!(matches!(error, WebDriverError::NoSuchElement(_)));
        driver.quit().await?;
        Ok::<_, anyhow::Error>(())
    })
    .await
    .context("WebDriver contract timed out")?;

    let _ = shutdown_tx.send(());
    server.await??;
    result.map_err(|error| anyhow::anyhow!("WebDriver contract failed: {error}"))?;

    let records = log
        .0
        .lock()
        .map_err(|_| anyhow::anyhow!("request log poisoned"))?
        .clone();
    let paths = records
        .iter()
        .map(|(path, _)| path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        [
            "POST /session",
            "POST /session/session-contract/timeouts",
            "POST /session/session-contract/url",
            "GET /session/session-contract/url",
            "POST /session/session-contract/element",
            "DELETE /session/session-contract",
        ]
    );
    assert!(records
        .first()
        .and_then(|(_, body)| body.get("capabilities"))
        .is_some());
    assert_eq!(
        records
            .get(1)
            .and_then(|(_, body)| body.get("script"))
            .and_then(Value::as_u64),
        Some(60_000)
    );
    assert_eq!(
        records
            .get(2)
            .and_then(|(_, body)| body.get("url"))
            .and_then(Value::as_str),
        Some("https://example.test/posted/✓")
    );
    Ok(())
}
