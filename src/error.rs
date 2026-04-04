use std::fmt;

/// Errors that can occur during magnifier operation.
#[derive(Debug)]
pub enum MagnifierError {
    /// A required Wayland global is not available
    WaylandGlobalMissing {
        /// Name of the missing global (e.g. "wl_compositor")
        global: &'static str,
    },
    /// Failed to create a shared memory file descriptor
    ShmFdCreateFailed,
}

impl fmt::Display for MagnifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MagnifierError::WaylandGlobalMissing { global } => {
                write!(
                    f,
                    "Required Wayland global '{global}' is not available. \
                     Your compositor may not support it."
                )
            }
            MagnifierError::ShmFdCreateFailed => {
                write!(f, "Failed to create memfd for shared memory")
            }
        }
    }
}

impl std::error::Error for MagnifierError {}

/// Convenience Result type for magnifier operations.
pub type Result<T> = std::result::Result<T, MagnifierError>;
