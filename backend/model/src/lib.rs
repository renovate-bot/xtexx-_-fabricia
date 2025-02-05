//! Database models for Fabricia backend.
//!
//! ## Primary Key Uniqueness
//! All primary keys should be as unique as possible,
//! in order to avoid conflicts with all historical IDs.

pub mod branch;
pub mod bus;
pub mod db;
pub mod job;
pub mod package;
pub mod target;
