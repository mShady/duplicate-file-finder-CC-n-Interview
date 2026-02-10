//! File scanning module

pub mod detector;
pub mod hasher;
pub mod types;
pub mod walker;

#[cfg(test)]
mod tests;

pub use detector::{DetectionResult, DetectionStats, DuplicateDetector, DuplicateFile, DuplicateGroup};
pub use hasher::{FileHasher, HashError, HashResult};
pub use types::*;
pub use walker::DirectoryWalker;
