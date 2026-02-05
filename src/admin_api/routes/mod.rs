// Admin API Routes
//
// All endpoints require authentication and internal network access

pub mod blocks;
pub mod dashboard;
pub mod config;
pub mod miners;
pub mod monitoring;
pub mod notifications;
pub mod payments;
pub mod workers;

use super::error::AdminError;
use super::AdminState;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

// Re-export submodules
pub use blocks::*;
pub use dashboard::*;
pub use config::*;
pub use miners::*;
pub use monitoring::*;
pub use notifications::*;
pub use payments::*;
pub use workers::*;
