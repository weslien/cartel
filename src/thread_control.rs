use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::thread;

/// Struct to check execution status of spawned thread.
#[derive(Debug)]
pub struct Flag {
    alive: Arc<AtomicBool>,
    interrupt: Arc<AtomicBool>,
}

impl Drop for Flag {
    fn drop(&mut self) {
        if thread::panicking() {
            (*self.interrupt).store(true, Ordering::Relaxed)
        }
    }
}

#[allow(clippy::new_without_default)]
impl Flag {
    /// Creates new flag.
    pub fn new() -> Self {
        Flag {
            alive: Arc::new(AtomicBool::new(true)),
            interrupt: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Creates new `Control` to control this flag.
    pub fn take_control(&self) -> Control {
        Control {
            alive: Arc::downgrade(&self.alive),
            interrupt: self.interrupt.clone(),
        }
    }

    /// Check the flag isn't stopped or interrupted.
    ///
    /// # Panics
    ///
    /// This method panics, if interrupt flag was set.
    pub fn alive(&self) -> bool {
        if (*self.interrupt).load(Ordering::Relaxed) {
            panic!("thread interrupted by thread-contol");
        }
        (*self.alive).load(Ordering::Relaxed)
    }

    /// Check the flag is not stopped and not interrupted
    /// Use it if panic is not desirable behavior
    pub fn is_alive(&self) -> bool {
        (*self.alive).load(Ordering::Relaxed)
            && !(*self.interrupt).load(Ordering::Relaxed)
    }

    /// Set interrupt flag and drop the instance
    pub fn interrupt(self) {
        (self.interrupt).store(true, Ordering::Relaxed)
    }
}

/// Struct to control thread execution.
#[derive(Debug, Clone)]
pub struct Control {
    alive: Weak<AtomicBool>,
    interrupt: Arc<AtomicBool>,
}

impl Control {
    /// Interrupt execution of thread.
    /// Actually it panics when thread checking flag.
    pub fn interrupt(&self) {
        (*self.interrupt).store(true, Ordering::Relaxed)
    }

    /// Set stop flag.
    pub fn stop(&self) {
        if let Some(flag) = self.alive.upgrade() {
            (*flag).store(false, Ordering::Relaxed)
        }
    }

    /// Return `true` if thread ended.
    pub fn is_done(&self) -> bool {
        self.alive.upgrade().is_none()
    }

    /// Return `true` if thread was interrupted or panicked.
    pub fn is_interrupted(&self) -> bool {
        (*self.interrupt).load(Ordering::Relaxed)
    }
}

/// Makes pair with connected flag and control.
pub fn make_pair() -> (Flag, Control) {
    let flag = Flag::new();
    let control = flag.take_control();
    (flag, control)
}
