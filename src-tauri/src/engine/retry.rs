use std::future::Future;
use std::time::Duration;

pub const REMOTE_MAX_ATTEMPTS: u32 = 3;
pub const REMOTE_BASE_BACKOFF_MS: u64 = 800;

#[derive(Debug, Copy, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_backoff_ms: u64,
}

pub const REMOTE_RETRY_POLICY: RetryPolicy = RetryPolicy {
    max_attempts: REMOTE_MAX_ATTEMPTS,
    base_backoff_ms: REMOTE_BASE_BACKOFF_MS,
};

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
    backoff_delay_with_policy(REMOTE_RETRY_POLICY, attempt)
}

pub fn backoff_delay_with_policy(policy: RetryPolicy, attempt: u32) -> Duration {
    let safe_attempt = attempt.max(1) as u64;
    Duration::from_millis(policy.base_backoff_ms * safe_attempt)
}

pub fn should_retry_remote_error(attempt: u32, message: &str) -> bool {
    attempt < REMOTE_RETRY_POLICY.max_attempts && is_transient_error_message(message)
}

pub async fn run_remote_with_retry<T, F, Fut>(
    operation_label: &str,
    mut operation: F,
) -> anyhow::Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = anyhow::Result<T>>,
{
    let mut attempt = 1u32;
    loop {
        match operation().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                let msg = e.to_string();
                if !should_retry_remote_error(attempt, &msg) {
                    return Err(e);
                }

                let delay = backoff_delay_with_policy(REMOTE_RETRY_POLICY, attempt);
                log::warn!(
                    "{} transient failure (attempt {}/{}), retrying in {:?}: {}",
                    operation_label,
                    attempt,
                    REMOTE_RETRY_POLICY.max_attempts,
                    delay,
                    msg
                );
                tokio::time::sleep(delay).await;
                attempt += 1;
            }
        }
    }
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

    #[test]
    fn retry_decision_respects_attempt_limit() {
        assert!(should_retry_remote_error(1, "HTTP 503 Service Unavailable"));
        assert!(!should_retry_remote_error(
            REMOTE_RETRY_POLICY.max_attempts,
            "HTTP 503 Service Unavailable"
        ));
        assert!(!should_retry_remote_error(1, "permission denied"));
    }
}
