use crate::AppState;
use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;
use zksync_os_types::{NotAcceptingReason, TransactionAcceptanceState};

#[derive(Serialize)]
pub struct AcceptingResponse {
    pub reasons: Vec<NotAcceptingReason>,
}

pub(crate) async fn accepting(
    State(state): State<AppState>,
) -> (StatusCode, Json<AcceptingResponse>) {
    let mut reasons = match state.acceptance_state.borrow().clone() {
        TransactionAcceptanceState::NotAccepting(reasons) => reasons,
        TransactionAcceptanceState::Accepting => vec![],
    };

    if *state.stop_receiver.borrow() {
        reasons.push(NotAcceptingReason::Terminating);
    }

    let http_status = if reasons.is_empty() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (http_status, Json(AcceptingResponse { reasons }))
}
