//! File scanning module

#![allow(unused_imports)] // Public API re-exports used in future phases

pub mod detector;
pub mod hasher;
pub mod types;
pub mod walker;

#[cfg(test)]
mod tests;

pub use detector::{DetectionResult, DetectionStats, DuplicateDetector, DuplicateFile, DuplicateGroup};
pub use hasher::{FileHasher, HashError};
pub use types::*;
pub use walker::DirectoryWalker;
