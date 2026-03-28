use std::time::Duration;

pub const REMOTE_MAX_ATTEMPTS: u32 = 3;
pub const REMOTE_BASE_BACKOFF_MS: u64 = 800;

pub fn is_transient_error_message(message: &str) -> bool {
    let msg = message.to_lowercase();
    [
        "timeout",
        "timed out",
        "temporarily unavailable",
        "try again",
        "connection reset",
        "connection aborted",
        "broken pipe",
        "eof",
        "network is unreachable",
        "connection refused",
        "dns",
        "name or service not known",
        "too many requests",
        "rate limit",
        "429",
        "502",
        "503",
        "504",
    ]
    .iter()
    .any(|needle| msg.contains(needle))
}

pub fn backoff_delay(attempt: u32) -> Duration {
    let safe_attempt = attempt.max(1) as u64;
    Duration::from_millis(REMOTE_BASE_BACKOFF_MS * safe_attempt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transient_detection_matches_expected_errors() {
        assert!(is_transient_error_message(
            "request timeout while uploading"
        ));
        assert!(is_transient_error_message("HTTP 503 Service Unavailable"));
        assert!(!is_transient_error_message("permission denied"));
        assert!(!is_transient_error_message("wrong password"));
    }

    #[test]
    fn backoff_is_monotonic() {
        let d1 = backoff_delay(1);
        let d2 = backoff_delay(2);
        let d3 = backoff_delay(3);
        assert!(d1 < d2 && d2 < d3);
    }
}
