use crate::PaneId;
use gpui::App;
#[cfg(not(target_family = "wasm"))]
use std::sync::Arc;

/// Window capabilities configured by the application host.
///
/// Native compilation alone does not imply that detach is wired up. Construct
/// this value from the service actually installed by the host.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WindowCapabilities {
    pub detached_windows: bool,
}
impl WindowCapabilities {
    /// The portable default: no detached-window host service is installed.
    pub const fn current() -> Self {
        Self {
            detached_windows: false,
        }
    }

    pub fn for_service(service: &dyn DetachedWindowService) -> Self {
        Self {
            detached_windows: service.is_available(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DetachError {
    Unavailable,
    Refused(String),
}
impl std::fmt::Display for DetachError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable => f.write_str("detached windows are unavailable in this host"),
            Self::Refused(reason) => write!(f, "detach refused: {reason}"),
        }
    }
}
impl std::error::Error for DetachError {}

/// Optional host-owned capability. It receives the GPUI [`App`] so a native
/// implementation can actually call `open_window`; the shared Mullion view
/// remains independent of window policy.
pub trait DetachedWindowService: Send + Sync {
    fn is_available(&self) -> bool;
    fn detach(&self, pane: &PaneId, cx: &mut App) -> Result<(), DetachError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableDetachedWindows;
impl DetachedWindowService for UnavailableDetachedWindows {
    fn is_available(&self) -> bool {
        false
    }

    fn detach(&self, _: &PaneId, _: &mut App) -> Result<(), DetachError> {
        Err(DetachError::Unavailable)
    }
}

/// A desktop host adapter backed by an application-provided window-opening
/// callback. It is absent on wasm, where one GPUI canvas owns the document.
#[cfg(not(target_family = "wasm"))]
#[derive(Clone)]
pub struct NativeDetachedWindowService {
    open: Arc<DetachCallback>,
}

#[cfg(not(target_family = "wasm"))]
type DetachCallback = dyn Fn(&PaneId, &mut App) -> Result<(), DetachError> + Send + Sync + 'static;

#[cfg(not(target_family = "wasm"))]
impl NativeDetachedWindowService {
    pub fn new(
        open: impl Fn(&PaneId, &mut App) -> Result<(), DetachError> + Send + Sync + 'static,
    ) -> Self {
        Self {
            open: Arc::new(open),
        }
    }
}

#[cfg(not(target_family = "wasm"))]
impl DetachedWindowService for NativeDetachedWindowService {
    fn is_available(&self) -> bool {
        true
    }

    fn detach(&self, pane: &PaneId, cx: &mut App) -> Result<(), DetachError> {
        (self.open)(pane, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_default_does_not_claim_host_integration() {
        assert!(!WindowCapabilities::current().detached_windows);
        assert!(!WindowCapabilities::for_service(&UnavailableDetachedWindows).detached_windows);
    }

    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn native_capability_is_service_derived() {
        let service = NativeDetachedWindowService::new(|_, _| Ok(()));
        assert!(WindowCapabilities::for_service(&service).detached_windows);
    }
}
