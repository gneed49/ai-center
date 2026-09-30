//! Explicit, bounded input from existing work tools. Not wired to routes yet.
//! Readers know neither the database nor agents; the service owns authorization.
pub mod locator;
pub mod models;
pub mod read;
pub mod reader;

mod admission;
pub mod commands;
mod persistence;
