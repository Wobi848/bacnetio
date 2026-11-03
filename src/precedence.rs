//! Requests a user waits for go before the reading in the background. Through a router to MS/TP
//! one request at a time gets through, so the details of a point took turns with the reading of
//! the list and came only after several of its requests (field test 3 November).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::sync::Notify;

#[derive(Clone, Default)]
pub struct Precedence {
    inner: Arc<Inner>,
}

#[derive(Default)]
struct Inner {
    running: AtomicUsize,
    done: Notify,
}

/// Held while a user's request runs; the reading in the background waits meanwhile.
pub struct First {
    inner: Arc<Inner>,
}

impl Precedence {
    pub fn first(&self) -> First {
        self.inner.running.fetch_add(1, Ordering::SeqCst);
        First {
            inner: Arc::clone(&self.inner),
        }
    }

    /// Returns once no user's request runs. Called by the reading in the background before each of
    /// its requests, so it gives way between two of them, never in the middle of one.
    pub async fn turn(&self) {
        loop {
            let done = self.inner.done.notified();
            tokio::pin!(done);
            // registered before the count is looked at, so an end in between is not missed
            done.as_mut().enable();
            if self.inner.running.load(Ordering::SeqCst) == 0 {
                return;
            }
            done.await;
        }
    }
}

impl Drop for First {
    fn drop(&mut self) {
        if self.inner.running.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.inner.done.notify_waiters();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn the_background_waits_while_a_user_request_runs() {
        let precedence = Precedence::default();
        tokio::time::timeout(Duration::from_millis(50), precedence.turn())
            .await
            .expect("free at once when nothing runs");

        let first = precedence.first();
        let second = precedence.first();
        let waiting = tokio::spawn({
            let precedence = precedence.clone();
            async move { precedence.turn().await }
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(!waiting.is_finished(), "waits while user requests run");
        drop(first);
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(!waiting.is_finished(), "still one running");
        drop(second);
        tokio::time::timeout(Duration::from_millis(500), waiting)
            .await
            .expect("goes on once none runs")
            .unwrap();
    }
}
