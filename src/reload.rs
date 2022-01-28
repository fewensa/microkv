use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc;
use std::time::Duration;

use crate::history::MicroKV030;
use crate::{errors, helpers};

/// Watch persist file and reload
pub struct WatchAndReload;

impl WatchAndReload {
    pub fn start(kv: MicroKV030) {
        std::thread::spawn(move || {
            if let Err(e) = Self::run(kv) {
                log::error!(
                    target: "microkv",
                    "Can not start MicroKV watch thread: [{:?}] {}",
                    e.error,
                    e.msg.unwrap_or("No more error message".to_string())
                );
            }
        });
    }

    fn run(kv: MicroKV030) -> errors::Result<()> {
        // Create a channel to receive the events.
        let (tx, rx) = mpsc::channel();

        // Automatically select the best implementation for your platform.
        // You can also access each implementation directly e.g. INotifyWatcher.
        let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(2))?;

        watcher.watch(kv.path.clone(), RecursiveMode::Recursive)?;

        // This is a simple loop, but you may want to use more complex logic here,
        // for example to handle I/O.
        loop {
            match rx.recv() {
                Ok(event) => match event {
                    DebouncedEvent::NoticeWrite(_) => {
                        if let Ok(v) = helpers::read_file_and_deserialize_bincode(&kv.path) {
                            match kv.replace(v) {
                                Ok(_) => log::info!(target: "microkv", "Reload data from file"),
                                Err(e) => {
                                    log::error!(
                                        target: "microkv",
                                        "Failed to reload data. [{:?}] {}",
                                        e.error,
                                        e.msg.unwrap_or("No more error message".to_string())
                                    )
                                }
                            }
                        }
                    }
                    _ => {}
                },
                Err(e) => {
                    log::error!(
                        target: "microkv",
                        "Watch db file error: {:?}",
                        e
                    );
                }
            }
        }
    }
}
