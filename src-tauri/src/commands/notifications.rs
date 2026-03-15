use tauri::State;
use crate::AppState;
use crate::db::queries;

/// Sends a test email to verify the Resend email notification setup.
#[tauri::command]
pub async fn send_test_email(
    state: State<'_, AppState>,
    to: String,
) -> Result<(), String> {
    // Save the address while we're at it
    queries::upsert_setting(&state.db, "notification_email", to.trim())
        .await
        .map_err(|e| e.to_string())?;

    crate::notifications::send_test_email(to.trim()).await
}
