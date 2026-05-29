//! Filesystem watch for the workspace directory, surfaced as an iced `Subscription`.
//!
//! `notify` runs its watcher on a background thread and invokes a callback per event; we forward a
//! unit signal into an async channel, debounce a burst into a single notification, and emit it. The
//! app reloads the workspace on each signal — but only applies the result when the on-disk tree
//! actually differs from what is in memory, so our own saves do not cause a reload loop.

use std::path::PathBuf;
use std::pin::Pin;
use std::time::Duration;

use iced::Subscription;
use iced::futures::{SinkExt, Stream};

/// Watch `dir` recursively; the subscription emits `()` (debounced) on each external change.
pub fn watch(dir: &std::path::Path) -> Subscription<()> {
    Subscription::run_with(dir.to_path_buf(), build_stream)
}

type EventStream = Pin<Box<dyn Stream<Item = ()> + Send>>;

// The `&PathBuf` arg is dictated by `Subscription::run_with`'s `fn(&D)` contract (here D = PathBuf).
#[allow(clippy::ptr_arg)]
fn build_stream(dir: &PathBuf) -> EventStream {
    let dir = dir.clone();
    Box::pin(iced::stream::channel(8, async move |mut output| {
        if !dir.exists() {
            return;
        }
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
        let handler = move |res: Result<notify::Event, notify::Error>| {
            if res.is_ok() {
                let _ = tx.send(());
            }
        };
        let mut watcher = match notify::recommended_watcher(handler) {
            Ok(w) => w,
            Err(_) => return,
        };
        if notify::Watcher::watch(&mut watcher, &dir, notify::RecursiveMode::Recursive).is_err() {
            return;
        }

        loop {
            if rx.recv().await.is_none() {
                return;
            }
            // Coalesce a burst of events: keep draining until the channel is quiet for 300ms.
            loop {
                match tokio::time::timeout(Duration::from_millis(300), rx.recv()).await {
                    Ok(Some(())) => continue,
                    Ok(None) => return,
                    Err(_) => break,
                }
            }
            if output.send(()).await.is_err() {
                return;
            }
        }
    }))
}
