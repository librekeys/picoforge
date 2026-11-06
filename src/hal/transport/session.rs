//! Serialize applet selection and command sequences within PicoForge.
//!
//! PC/SC shared connections do not give each handle an independent selected
//! applet. Hold this guard from before SELECT until after the card disconnects,
//! including response paging and VERIFY/write sequences. Both Rescue and CCID
//! must participate. This is process-local; it does not exclude other programs.

use std::sync::{Mutex, MutexGuard};

static SESSION: Mutex<()> = Mutex::new(());

pub(super) fn acquire() -> MutexGuard<'static, ()> {
    // No Rust data is protected. Every new session selects its applet again,
    // so recovering after a failed worker is safe.
    SESSION
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::{SESSION, acquire};

    #[test]
    fn applet_sessions_exclude_other_workers_and_release_on_error() {
        let first = acquire();
        std::thread::spawn(|| {
            assert!(matches!(
                SESSION.try_lock(),
                Err(std::sync::TryLockError::WouldBlock)
            ));
        })
        .join()
        .unwrap();
        drop(first);

        fn failed_open() -> Result<(), ()> {
            let _session = acquire();
            Err(())
        }
        assert!(failed_open().is_err());
        let next = SESSION
            .try_lock()
            .expect("failed open must release the session");
        drop(next);
    }
}
