mod adapter;
pub mod domain;
pub mod provider;
pub mod storage;
pub use adapter::Adapter;
#[cfg(test)]
mod qa_tests;
