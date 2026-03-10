pub use vex_hardware::{id, keystore, tpm, traits};
pub mod cloud;
pub mod error;
pub mod ffi;
pub mod kernel;
pub mod persist;
pub mod runtime;
pub mod zk;

pub use cloud::adapter::ProvnAnchor;
pub use cloud::client::ProvnCloudClient;
pub use config::AttestConfig;
pub use error::AttestError;
pub use id::AttestAgent;
pub use keystore::KeyManager;
pub use persist::audit::{ActorType, AuditEvent, AuditEventType, AuditStore};
pub use persist::sqlite::LocalStore;
pub use runtime::intent::{Intent, IntentStatus};
pub use runtime::interceptor::AttestTerminalInterceptor;
pub use runtime::policy::{ActionContext, Policy, PolicyEngine};
pub use runtime::watcher::AttestWatcher;

pub mod config;
