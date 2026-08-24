//! Credential storage as a port (ADR-0008).
//!
//! Tokens never go into the application database. They go here, and the concrete
//! backend is the OS keychain wherever one exists.

mod key;
mod memory;
mod secret;
mod store;

#[cfg(feature = "testing")]
pub mod contract;

pub use key::SecretKey;
pub use memory::MemorySecretStore;
pub use secret::Secret;
pub use store::SecretStore;
