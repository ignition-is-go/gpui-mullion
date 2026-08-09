use crate::PaneId;

/// Runtime-relevant window capabilities. Mullion itself always renders every workspace
/// inside the document-owned root window; detaching is an optional host service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowCapabilities {
    pub detached_windows: bool,
}
impl WindowCapabilities {
    pub const fn current() -> Self {
        Self {
            detached_windows: cfg!(not(target_family = "wasm")),
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
            Self::Unavailable => f.write_str("detached windows are unavailable on this platform"),
            Self::Refused(reason) => write!(f, "detach refused: {reason}"),
        }
    }
}
impl std::error::Error for DetachError {}

/// Optional host-owned desktop capability. A service may open a second OS window and
/// transfer host content; browser hosts should use [`UnavailableDetachedWindows`].
pub trait DetachedWindowService: Send + Sync {
    fn detach(&self, pane: &PaneId) -> Result<(), DetachError>;
}
#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableDetachedWindows;
impl DetachedWindowService for UnavailableDetachedWindows {
    fn detach(&self, _: &PaneId) -> Result<(), DetachError> {
        Err(DetachError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unavailable_service_is_graceful() {
        assert_eq!(
            UnavailableDetachedWindows.detach(&PaneId::new("pane")),
            Err(DetachError::Unavailable)
        );
    }
    #[test]
    fn capability_matches_target() {
        assert_eq!(
            WindowCapabilities::current().detached_windows,
            cfg!(not(target_family = "wasm"))
        );
    }
}
