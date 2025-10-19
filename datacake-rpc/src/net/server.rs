use std::convert::Infallible;
use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use http::{Request, Response, StatusCode};
use hyper::server::conn::http2;
use hyper::service::service_fn;
use rkyv::AlignedVec;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tracing::{error, warn};

use crate::body::Body;
use crate::server::ServerState;
use crate::Status;

/// Starts the RPC server.
///
/// This takes a binding socket address and server state.
pub(crate) async fn start_rpc_server(
    bind_addr: SocketAddr,
    state: ServerState,
) -> io::Result<JoinHandle<()>> {
    #[cfg(not(feature = "simulation"))]
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    #[cfg(feature = "simulation")]
    let listener = turmoil::net::TcpListener::bind(bind_addr).await?;

    let (ready, waiter) = oneshot::channel();
    let handle = tokio::spawn(async move {
        let _ = ready.send(());

        loop {
            let (io, remote_addr) = match listener.accept().await {
                Ok(accepted) => accepted,
                Err(e) => {
                    warn!(error = ?e, "Failed to accept client.");
                    continue;
                },
            };

            let state = state.clone();
            tokio::task::spawn(async move {
                let state = state.clone();
                let handler = service_fn(move |req| {
                    handle_connection(req, state.clone(), remote_addr)
                });

                let connection = http2::Builder::new(hyper_util::rt::TokioExecutor::new())
                    .adaptive_window(true)
                    .keep_alive_timeout(Duration::from_secs(10))
                    .serve_connection(hyper_util::rt::TokioIo::new(io), handler);

                if let Err(e) = connection.await {
                    error!(error = ?e, "Error while serving HTTP connection.");
                }
            });
        }
    });

    let _ = waiter.await;

    Ok(handle)
}

/// A single connection handler.
///
/// This accepts new streams being created and spawns concurrent tasks to handle
/// them.
async fn handle_connection(
    req: Request<hyper::body::Incoming>,
    state: ServerState,
    remote_addr: SocketAddr,
) -> Result<Response<http_body_util::Full<bytes::Bytes>>, Infallible> {
    match handle_message(req, state, remote_addr).await {
        Ok(r) => Ok(r),
        Err(e) => {
            let mut response = Response::new(http_body_util::Full::new(bytes::Bytes::from(e.to_string())));
            (*response.status_mut()) = StatusCode::INTERNAL_SERVER_ERROR;
            Ok(response)
        },
    }
}

async fn handle_message(
    req: Request<hyper::body::Incoming>,
    state: ServerState,
    remote_addr: SocketAddr,
) -> anyhow::Result<Response<http_body_util::Full<bytes::Bytes>>> {
    let reply = try_handle_request(req, state, remote_addr).await;

    match reply {
        Ok(body) => {
            // Convert the body to bytes for the response
            let bytes = match body.collect().await {
                Ok(bytes) => bytes,
                Err(_) => bytes::Bytes::new(),
            };
            let mut response = Response::new(http_body_util::Full::new(bytes));
            (*response.status_mut()) = StatusCode::OK;
            Ok(response)
        },
        Err(status) => Ok(create_bad_request(&status)),
    }
}

async fn try_handle_request(
    req: Request<hyper::body::Incoming>,
    state: ServerState,
    remote_addr: SocketAddr,
) -> Result<Body, Status> {
    let (req, body) = req.into_parts();
    let uri = req.uri.path();
    let headers = req.headers;

    let handler = state
        .get_handler(uri)
        .ok_or_else(|| Status::unavailable(format!("Unknown service {uri}")))?;

    handler
        .try_handle(remote_addr, headers, Body::new(body))
        .await
}

fn create_bad_request(status: &Status) -> Response<http_body_util::Full<bytes::Bytes>> {
    // This should be infallible.
    let buffer =
        crate::rkyv_tooling::to_view_bytes(status).unwrap_or_else(|_| AlignedVec::new());

    let mut response = Response::new(http_body_util::Full::new(bytes::Bytes::from(buffer.to_vec())));
    (*response.status_mut()) = StatusCode::BAD_REQUEST;

    response
}
