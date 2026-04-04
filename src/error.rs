use std::fmt;

/// Errors that can occur during magnifier operation.
#[allow(dead_code)]
#[derive(Debug)]
pub enum MagnifierError {
    /// Failed to connect to the Wayland compositor
    WaylandConnection,
    /// A required Wayland global is not available
    WaylandGlobalMissing {
        /// Name of the missing global (e.g. "wl_compositor")
        global: &'static str,
    },
    /// Failed to create a Wayland surface
    SurfaceCreateFailed,
    /// GPU adapter not found
    GpuAdapterNotFound,
    /// GPU device/queue initialization failed
    GpuInitFailed {
        detail: String,
    },
    /// Failed to create a Wayland surface for wgpu
    SurfaceCreateWgpuFailed {
        detail: String,
    },
    /// Failed to create a shared memory file descriptor
    ShmFdCreateFailed,
    /// Failed to resize the shared memory file
    FtruncateFailed,
    /// Failed to memory-map the file descriptor
    MmapFailed,
}

impl fmt::Display for MagnifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MagnifierError::WaylandConnection => {
                write!(f, "Failed to connect to Wayland. Is a compositor running?")
            }
            MagnifierError::WaylandGlobalMissing { global } => {
                write!(
                    f,
                    "Required Wayland global '{global}' is not available. \
                     Your compositor may not support it."
                )
            }
            MagnifierError::SurfaceCreateFailed => {
                write!(f, "Failed to create compositor surface")
            }
            MagnifierError::GpuAdapterNotFound => {
                write!(
                    f,
                    "No GPU adapter found. Install Vulkan drivers or enable lavapipe."
                )
            }
            MagnifierError::GpuInitFailed { detail } => {
                write!(f, "GPU device init failed: {detail}")
            }
            MagnifierError::SurfaceCreateWgpuFailed { detail } => {
                write!(f, "wgpu surface creation failed: {detail}")
            }
            MagnifierError::ShmFdCreateFailed => {
                write!(f, "Failed to create memfd for shared memory")
            }
            MagnifierError::FtruncateFailed => {
                write!(f, "Failed to resize shm fd")
            }
            MagnifierError::MmapFailed => {
                write!(f, "Failed to mmap shm fd")
            }
        }
    }
}

impl std::error::Error for MagnifierError {}

/// Convenience Result type for magnifier operations.
pub type Result<T> = std::result::Result<T, MagnifierError>;
