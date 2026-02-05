// DMPool Library
//
// This library provides shared functionality for the DMPool Bitcoin mining pool
// a derivative of Hydrapool by 256 Foundation.

pub mod alert;
pub mod admin_api;
pub mod auth;
pub mod audit;
pub mod backup;
pub mod bitcoin;
pub mod config;
pub mod config_mgt;
pub mod confirmation;
pub mod db;
pub mod health;
pub mod observer_api;
pub mod payment;
pub mod pplns_validator;
pub mod rate_limit;
pub mod two_factor;

pub use alert::{AlertManager, AlertConfig, AlertRule, AlertChannel, AlertLevel, AlertCondition, Alert};
pub use auth::{AuthManager, Claims, User, UserInfo, LoginRequest, LoginResponse, PasswordValidation, validate_password_strength};
pub use audit::{AuditLogger, AuditLog, AuditFilter, AuditStats};
pub use backup::{BackupManager, BackupConfig, BackupMetadata, BackupStats};
pub use bitcoin::{BitcoinRpcClient, BlockchainInfo, MempoolInfo, DecodedTransaction, TxInput, TxOutput, WalletInfo, UnspentOutput};
pub use config_mgt::{ConfigManager, ConfigVersion, ConfigDiff, ScheduledChange, ConfigSchema};
pub use confirmation::{ConfigConfirmation, ConfigChangeRequest, RiskLevel, ConfigMeta};
pub use db::{DatabaseManager, PoolStats, MinerStats, BlockInfo, BlockDetail};
pub use health::{HealthChecker, HealthStatus, ComponentStatus};
pub use observer_api::{self, ObserverState};
pub use payment::{PaymentManager, PaymentConfig, Payout, PayoutStatus, MinerBalance, PaymentStats};
pub use pplns_validator::{PplnsSimulator, PayoutCalculation, PplnsValidationResult, ScenarioResult};
pub use rate_limit::{RateLimiterState, RateLimitConfig, extract_client_ip};
pub use two_factor::{TwoFactorManager, TwoFactorSetup, TwoFactorVerify, TwoFactorEnable, TwoFactorStatus, TwoFactorLogin};

