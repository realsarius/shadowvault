use serde::Serialize;

#[derive(Debug, Copy, Clone, Serialize, specta::Type, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CommandErrorCode {
    BlockedPath,
    MissingSnapshot,
    WrongPassword,
    ChainIncomplete,
    IoFailure,
    VaultLocked,
    NotFound,
    InvalidInput,
    ConcurrencyConflict,
}

impl CommandErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            CommandErrorCode::BlockedPath => "blocked_path",
            CommandErrorCode::MissingSnapshot => "missing_snapshot",
            CommandErrorCode::WrongPassword => "wrong_password",
            CommandErrorCode::ChainIncomplete => "chain_incomplete",
            CommandErrorCode::IoFailure => "io_failure",
            CommandErrorCode::VaultLocked => "vault_locked",
            CommandErrorCode::NotFound => "not_found",
            CommandErrorCode::InvalidInput => "invalid_input",
            CommandErrorCode::ConcurrencyConflict => "concurrency_conflict",
        }
    }
}

#[derive(Debug, Serialize)]
struct CommandErrorPayload {
    error_code: CommandErrorCode,
    message: String,
}

pub fn command_error(error_code: CommandErrorCode, message: impl Into<String>) -> String {
    let payload = CommandErrorPayload {
        error_code,
        message: message.into(),
    };
    serde_json::to_string(&payload)
        .unwrap_or_else(|_| format!("{}: {}", payload.error_code.as_str(), payload.message))
}
