//! Opaque C ABI core handle.

use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak};

use aegis_core::executor::Executor;

static NEXT_CORE_ID: AtomicUsize = AtomicUsize::new(1);

thread_local! {
    static ACTIVE_CORE_LOCKS: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
}

pub(crate) enum CoreLockError {
    Poisoned,
    Reentrant,
}

/// Opaque core handle used by C ABI consumers.
///
/// FFI calls on one core handle are serialized internally. Hosts must not call
/// back into the same core handle reentrantly from a command callback; defer
/// that work or use a separate core handle.
pub struct AegisCoreHandle {
    state: Arc<AegisCoreState>,
}

pub(crate) struct AegisCoreState {
    id: usize,
    executor: Mutex<Executor>,
}

impl AegisCoreHandle {
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(AegisCoreState {
                id: NEXT_CORE_ID.fetch_add(1, Ordering::Relaxed),
                executor: Mutex::new(Executor::with_builtins()),
            }),
        }
    }

    pub(crate) fn id(&self) -> usize {
        self.state.id()
    }

    pub(crate) fn executor(&self) -> Result<AegisExecutorGuard<'_>, CoreLockError> {
        self.state.executor()
    }

    pub(crate) fn downgrade(&self) -> Weak<AegisCoreState> {
        Arc::downgrade(&self.state)
    }
}

impl AegisCoreState {
    pub(crate) fn id(&self) -> usize {
        self.id
    }

    pub(crate) fn executor(&self) -> Result<AegisExecutorGuard<'_>, CoreLockError> {
        if ACTIVE_CORE_LOCKS.with(|locks| locks.borrow().contains(&self.id)) {
            return Err(CoreLockError::Reentrant);
        }

        let guard = self.executor.lock().map_err(|_| CoreLockError::Poisoned)?;
        ACTIVE_CORE_LOCKS.with(|locks| locks.borrow_mut().push(self.id));
        Ok(AegisExecutorGuard {
            core_id: self.id,
            guard,
        })
    }
}

pub(crate) struct AegisExecutorGuard<'core> {
    core_id: usize,
    guard: MutexGuard<'core, Executor>,
}

impl Deref for AegisExecutorGuard<'_> {
    type Target = Executor;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl DerefMut for AegisExecutorGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

impl Drop for AegisExecutorGuard<'_> {
    fn drop(&mut self) {
        ACTIVE_CORE_LOCKS.with(|locks| {
            let mut locks = locks.borrow_mut();
            if let Some(index) = locks.iter().rposition(|core_id| *core_id == self.core_id) {
                locks.remove(index);
            }
        });
    }
}
