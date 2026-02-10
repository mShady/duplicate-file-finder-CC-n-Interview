//! File scanning module

pub mod hasher;
pub mod types;
pub mod walker;

#[cfg(test)]
mod tests;

pub use hasher::{FileHasher, HashError, HashResult};
pub use types::*;
pub use walker::DirectoryWalker;
