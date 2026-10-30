//! Default-on, authenticated loopback UI. The browser is a bounded client of the
//! same runtime API; it never owns jobs or runner monitoring.
use crate::{ipc::Request, storage::Store};
use anyhow::Result;
use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Query, State},
    http::{HeaderValue, Request as HttpRequest, StatusCode},
    middleware::{self, Next},
    response::{
        IntoResponse, Response, Sse,
        sse::{Event, KeepAlive},
    },
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::json;
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio::sync::{Mutex, Semaphore, mpsc, watch};
use tokio_stream::wrappers::ReceiverStream;
mod sampling;
include!(concat!(env!("OUT_DIR"), "/web_assets.rs"));
#[derive(Clone)]
struct Web {
    store: Store,
    token: String,
    host: String,
    requests: Arc<Semaphore>,
    streams: Arc<Semaphore>,
    samples: Arc<Mutex<sampling::Sampler>>,
    stopped: watch::Receiver<bool>,
}
pub(crate) async fn start(
    store: Store,
    stopped: watch::Receiver<bool>,
) -> Result<Option<tokio::task::JoinHandle<()>>> {
    if std::env::var("AJR_WEB_UI").ok().as_deref() == Some("off") {
        *store
            .web_info
            .lock()
            .map_err(|_| anyhow::anyhow!("UI state poisoned"))? = json!({"enabled":false});
        return Ok(None);
    }
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;
    let host = listener.local_addr()?.to_string();
    let token = format!("{}{}", crate::core::id(), crate::core::id());
    let url = format!("http://{host}/");
    *store
        .web_info
        .lock()
        .map_err(|_| anyhow::anyhow!("UI state poisoned"))? = json!({"enabled":true,"url":url,"access_url":format!("{url}#token={token}"),"authentication":"ephemeral bearer token"});
    let state = Web {
        samples: Arc::new(Mutex::new(sampling::Sampler::new(store.clone()))),
        store,
        token,
        host,
        requests: Arc::new(Semaphore::new(16)),
        streams: Arc::new(Semaphore::new(sampling::MAX_VIEWERS)),
        stopped: stopped.clone(),
    };
    let app = Router::new()
        .route("/api/request", post(request))
        .route("/api/events", get(events))
        .fallback(asset)
        .layer(DefaultBodyLimit::max(131072))
        .layer(middleware::from_fn_with_state(state.clone(), protect))
        .with_state(state);
    Ok(Some(tokio::spawn(async move {
        let mut stopped = stopped;
        if let Err(error) = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                if !*stopped.borrow() {
                    let _ = stopped.changed().await;
                }
            })
            .await
        {
            tracing::error!(error=%error,"Web UI stopped");
        }
    })))
}
fn error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({"error":message}))).into_response()
}
async fn protect(State(state): State<Web>, req: HttpRequest<Body>, next: Next) -> Response {
    if req.headers().get("host").and_then(|v| v.to_str().ok()) != Some(state.host.as_str()) {
        return error(StatusCode::FORBIDDEN, "Invalid local host");
    }
    if let Some(origin) = req.headers().get("origin")
        && origin.to_str().ok() != Some(format!("http://{}", state.host).as_str())
    {
        return error(StatusCode::FORBIDDEN, "Cross-origin access refused");
    }
    if req.uri().path().starts_with("/api/") {
        let auth = format!("Bearer {}", state.token);
        if req
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            != Some(auth.as_str())
        {
            return error(
                StatusCode::UNAUTHORIZED,
                "Run ui from the selected profile to obtain a fresh access URL",
            );
        }
    }
    let Ok(_permit) = state.requests.clone().try_acquire_owned() else {
        return error(StatusCode::TOO_MANY_REQUESTS, "Local request limit reached");
    };
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert("content-security-policy",HeaderValue::from_static("default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'"));
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert("cache-control", HeaderValue::from_static("no-store"));
    response
}
async fn request(State(state): State<Web>, Json(req): Json<Request>) -> Response {
    if !matches!(
        &req,
        Request::Inspect { .. }
            | Request::Monitor { .. }
            | Request::Stop { .. }
            | Request::Status { .. }
            | Request::Logs { .. }
            | Request::Events { .. }
            | Request::Doctor
            | Request::Deliveries { .. }
            | Request::DeliveryAttempts { .. }
    ) {
        return error(
            StatusCode::FORBIDDEN,
            "Operation is unavailable through the Web UI",
        );
    }
    match tokio::time::timeout(
        Duration::from_secs(35),
        crate::ipc::dispatch(&state.store, req),
    )
    .await
    {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(e)) => error(StatusCode::BAD_REQUEST, &format!("{e:#}")),
        Err(_) => error(
            StatusCode::GATEWAY_TIMEOUT,
            "Operation deadline exceeded; inspect its recorded result before retrying",
        ),
    }
}
#[derive(Deserialize)]
struct Watch {
    job_id: Option<String>,
}
async fn events(State(state): State<Web>, Query(query): Query<Watch>) -> Response {
    if let Some(job) = &query.job_id
        && crate::core::validate_id(job).is_err()
    {
        return error(StatusCode::BAD_REQUEST, "Invalid job ID");
    }
    let Ok(permit) = state.streams.clone().try_acquire_owned() else {
        return error(StatusCode::TOO_MANY_REQUESTS, "Viewer limit reached");
    };
    let (tx, rx) = mpsc::channel(1);
    tokio::spawn(async move {
        let _permit = permit;
        let mut stopped = state.stopped.clone();
        loop {
            let sample = async {
                state
                    .samples
                    .lock()
                    .await
                    .sample(query.job_id.clone())
                    .await
            };
            let value = tokio::select! {
                _=stopped.changed()=>break,
                _=tx.closed()=>break,
                result=sample=>match result {Ok(v)=>v,Err(e)=>json!({"error":format!("{e:#}")}).to_string().into()},
            };
            let _ = tx.try_send(Ok::<_, Infallible>(Event::default().data(value.as_ref())));
            tokio::select! {_=stopped.changed()=>break,_=tx.closed()=>break,_=tokio::time::sleep(Duration::from_secs(1))=>{}}
        }
    });
    Sse::new(ReceiverStream::new(rx))
        .keep_alive(KeepAlive::default())
        .into_response()
}
async fn asset(req: HttpRequest<Body>) -> Response {
    if req.method() != axum::http::Method::GET {
        return error(StatusCode::METHOD_NOT_ALLOWED, "GET required");
    }
    let path = if req.uri().path() == "/" {
        "/index.html"
    } else {
        req.uri().path()
    };
    let bytes = if path == "/icon.svg" {
        Some(include_bytes!("../assets/icons/agent-job-relay.svg").as_slice())
    } else {
        ASSETS
            .iter()
            .find(|(name, _)| *name == path)
            .map(|(_, bytes)| *bytes)
    };
    let Some(bytes) = bytes else {
        return error(StatusCode::NOT_FOUND, "Asset not found");
    };
    let mime = if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".js") {
        "text/javascript; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else {
        "application/octet-stream"
    };
    ([("content-type", mime)], bytes).into_response()
}
