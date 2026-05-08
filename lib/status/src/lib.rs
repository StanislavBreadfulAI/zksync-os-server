//! Status HTTP endpoints.
//!
//! - `GET /status/live` — liveness. Always 200 while the process is up.
//! - `GET /status/ready` — K8s-style readiness. 200 while serving; 503 during graceful
//!   shutdown. Safe to wire as a Kubernetes `readinessProbe`. Intentionally does NOT
//!   flip on acceptance state: RPC readers in this process keep serving while writes
//!   are paused, so readers must not be drained from service endpoints on a transient
//!   acceptance flip.
//! - `GET /status/accepting` — transaction acceptance gate. 200 when accepting, 503
//!   with structured `reasons` JSON when not. This is the endpoint to alert / dashboard
//!   on, NOT to wire as readinessProbe.
//! - `GET /status/pipeline` — per-component backpressure and lag snapshot for
//!   diagnostics and dashboards.

mod accepting;
mod pipeline;

use crate::accepting::accepting;
use crate::pipeline::pipeline;
use axum::{Router, extract::State, http::StatusCode, routing::get};
use reth_tasks::shutdown::GracefulShutdown;
use std::net::SocketAddr;
use tokio::{net::TcpListener, sync::watch};
use zksync_os_backpressure::PipelineSnapshot;
use zksync_os_types::TransactionAcceptanceState;

#[derive(Clone)]
pub struct StatusServerState {
    pub stop_receiver: watch::Receiver<bool>,
    pub acceptance_state: watch::Receiver<TransactionAcceptanceState>,
    pub pipeline_snapshot: watch::Receiver<PipelineSnapshot>,
}

pub(crate) type AppState = StatusServerState;

async fn live() -> StatusCode {
    StatusCode::OK
}

async fn ready(State(state): State<AppState>) -> StatusCode {
    if *state.stop_receiver.borrow() {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    }
}

pub async fn run_status_server(
    addr: SocketAddr,
    shutdown: GracefulShutdown,
    state: StatusServerState,
) {
    let app = Router::new()
        .route("/status/live", get(live))
        .route("/status/ready", get(ready))
        .route("/status/accepting", get(accepting))
        .route("/status/pipeline", get(pipeline))
        .with_state(state);

    let listener = TcpListener::bind(addr)
        .await
        .expect("cannot listen on address");

    let addr = listener.local_addr().expect("cannot get local address");
    tracing::info!(%addr, "status server running");

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let graceful_guard = shutdown.await;
            tracing::info!("status server graceful shutdown complete");
            drop(graceful_guard);
        })
        .await
        .expect("never errors according to doc");
}
