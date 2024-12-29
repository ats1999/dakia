pub mod config_store;
mod pattern_registry;

pub use pattern_registry::PatternRegistry;
// 4. thread local lazy config read and update

// 1. thread local lazy regex registry
// 2. thread local lazy filter registry
// 3. thread local lazy interceptor registry
