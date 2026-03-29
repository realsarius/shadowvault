use dashmap::{mapref::entry::Entry, DashMap};
use std::future::Future;
use std::sync::Arc;
use std::time::Instant;
use tokio::task::AbortHandle;

pub fn try_claim_destination(
    inflight_jobs: &Arc<DashMap<String, Instant>>,
    destination_id: &str,
) -> bool {
    match inflight_jobs.entry(destination_id.to_string()) {
        Entry::Occupied(_) => false,
        Entry::Vacant(v) => {
            v.insert(Instant::now());
            true
        }
    }
}

pub fn release_destination(inflight_jobs: &Arc<DashMap<String, Instant>>, destination_id: &str) {
    inflight_jobs.remove(destination_id);
}

pub fn spawn_tracked_job<F>(
    destination_id: String,
    running_jobs: Arc<DashMap<String, AbortHandle>>,
    inflight_jobs: Arc<DashMap<String, Instant>>,
    task: F,
) where
    F: Future<Output = ()> + Send + 'static,
{
    struct Cleanup {
        destination_id: String,
        running_jobs: Arc<DashMap<String, AbortHandle>>,
        inflight_jobs: Arc<DashMap<String, Instant>>,
    }

    impl Drop for Cleanup {
        fn drop(&mut self) {
            self.running_jobs.remove(&self.destination_id);
            self.inflight_jobs.remove(&self.destination_id);
        }
    }

    let task_dest = destination_id.clone();
    let task_running = running_jobs.clone();
    let task_inflight = inflight_jobs.clone();
    let join = tokio::task::spawn(async move {
        let _cleanup = Cleanup {
            destination_id: task_dest,
            running_jobs: task_running,
            inflight_jobs: task_inflight,
        };
        task.await;
    });

    running_jobs.insert(destination_id, join.abort_handle());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claim_destination_is_single_flight() {
        let inflight: Arc<DashMap<String, Instant>> = Arc::new(DashMap::new());
        assert!(try_claim_destination(&inflight, "dest-1"));
        assert!(!try_claim_destination(&inflight, "dest-1"));
        release_destination(&inflight, "dest-1");
        assert!(try_claim_destination(&inflight, "dest-1"));
    }
}
