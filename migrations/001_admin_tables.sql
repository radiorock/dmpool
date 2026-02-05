-- DMPool Admin Tables Migration
-- Version: 001
-- Description: Create tables for admin functionality
--
-- These tables extend Hydrapool's core functionality with:
-- - Miner management (ban/unban)
-- - Custom payment thresholds
-- - Notification configs
-- - Audit logging

-- Enable UUID extension if not exists
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ============================================================================
-- Banned Miners Table
-- ============================================================================
CREATE TABLE IF NOT EXISTS banned_miners (
    id SERIAL PRIMARY KEY,
    address VARCHAR(255) UNIQUE NOT NULL,
    banned_at TIMESTAMPTZ DEFAULT NOW(),
    banned_by VARCHAR(255) DEFAULT 'system',
    reason TEXT,
    is_permanent BOOLEAN DEFAULT false,
    expires_at TIMESTAMPTZ,  -- NULL if permanent
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index for faster lookups
CREATE INDEX idx_banned_miners_address ON banned_miners(address);
CREATE INDEX idx_banned_miners_is_active ON banned_miners(
    CASE
        WHEN is_permanent = true OR expires_at > NOW() THEN 1
        ELSE 0
    END
);

-- ============================================================================
-- Custom Payment Thresholds Table
-- ============================================================================
CREATE TABLE IF NOT EXISTS custom_thresholds (
    address VARCHAR(255) PRIMARY KEY,
    threshold_sats BIGINT NOT NULL DEFAULT 1000000,  -- 0.01 BTC
    min_payout_sats BIGINT NOT NULL DEFAULT 100000,   -- 0.001 BTC (manual)
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    updated_by VARCHAR(255) DEFAULT 'system'
);

-- Index for faster lookups
CREATE INDEX idx_custom_thresholds_address ON custom_thresholds(address);

-- ============================================================================
-- Notification Configs Table
-- ============================================================================
CREATE TABLE IF NOT EXISTS notification_configs (
    id SERIAL PRIMARY KEY,
    user_type VARCHAR(50) NOT NULL CHECK (user_type IN ('admin', 'miner')),
    address VARCHAR(255),  -- NULL for admin, required for miner
    telegram_enabled BOOLEAN DEFAULT false,
    telegram_chat_id VARCHAR(255),
    email_enabled BOOLEAN DEFAULT false,
    email_address VARCHAR(255),
    notify_block_found BOOLEAN DEFAULT true,
    notify_payment_received BOOLEAN DEFAULT true,
    notify_payment_confirmed BOOLEAN DEFAULT true,
    notify_system_alert BOOLEAN DEFAULT true,
    notify_miner_offline BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT unique_user_notification UNIQUE (user_type, address)
);

-- Indexes
CREATE INDEX idx_notification_configs_user_type ON notification_configs(user_type);
CREATE INDEX idx_notification_configs_address ON notification_configs(address);
CREATE INDEX idx_notification_configs_telegram ON notification_configs(telegram_enabled) WHERE telegram_enabled = true;
CREATE INDEX idx_notification_configs_email ON notification_configs(email_enabled) WHERE email_enabled = true;

-- ============================================================================
-- Notification History Table
-- ============================================================================
CREATE TABLE IF NOT EXISTS notification_history (
    id SERIAL PRIMARY KEY,
    config_id INTEGER REFERENCES notification_configs(id) ON DELETE SET NULL,
    notification_type VARCHAR(50) NOT NULL CHECK (notification_type IN (
        'block_found', 'payment_received', 'payment_confirmed', 'system_alert', 'miner_offline'
    )),
    channel VARCHAR(20) NOT NULL CHECK (channel IN ('telegram', 'email')),
    subject VARCHAR(500),
    content TEXT,
    status VARCHAR(20) NOT NULL CHECK (status IN ('pending', 'sent', 'failed')),
    sent_at TIMESTAMPTZ,
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_notification_history_config_id ON notification_history(config_id);
CREATE INDEX idx_notification_history_type ON notification_history(notification_type);
CREATE INDEX idx_notification_history_status ON notification_history(status);
CREATE INDEX idx_notification_history_created_at ON notification_history(created_at DESC);

-- ============================================================================
-- System Configs Table (Dynamic Configuration)
-- ============================================================================
CREATE TABLE IF NOT EXISTS system_configs (
    key VARCHAR(100) PRIMARY KEY,
    value TEXT NOT NULL,
    value_type VARCHAR(20) NOT NULL CHECK (value_type IN ('string', 'number', 'boolean', 'json')),
    description TEXT,
    category VARCHAR(50) DEFAULT 'general',  -- 'pool', 'payment', 'notification', etc.
    is_public BOOLEAN DEFAULT false,  -- Can be exposed in public API
    requires_reload BOOLEAN DEFAULT false,  -- Requires service reload to take effect
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    updated_by VARCHAR(255) DEFAULT 'system'
);

-- Index
CREATE INDEX idx_system_configs_category ON system_configs(category);

-- Insert default configs
INSERT INTO system_configs (key, value, value_type, description, category, is_public, requires_reload) VALUES
    ('pool.name', 'DMPool', 'string', 'Pool name', 'pool', true, false),
    ('pool.fee_percent', '1.0', 'number', 'Pool fee percentage (1.0 = 1%)', 'pool', true, false),
    ('pool.min_payout_sats', '1000000', 'number', 'Minimum payout threshold in satoshis', 'payment', false, false),
    ('pool.pplns_window_days', '7', 'number', 'PPLNS window in days', 'pool', true, false),
    ('notification.telegram.enabled', 'false', 'boolean', 'Telegram notifications enabled', 'notification', false, false),
    ('notification.email.enabled', 'false', 'boolean', 'Email notifications enabled', 'notification', false, false),
    ('admin.session_timeout_minutes', '60', 'number', 'Admin session timeout in minutes', 'admin', false, false)
ON CONFLICT (key) DO NOTHING;

-- ============================================================================
-- Admin Audit Logs Table (Extended)
-- ============================================================================
CREATE TABLE IF NOT EXISTS admin_audit_logs (
    id SERIAL PRIMARY KEY,
    admin_user VARCHAR(255) NOT NULL,
    action VARCHAR(100) NOT NULL,
    target_type VARCHAR(50),  -- 'miner', 'worker', 'config', 'payment', 'block'
    target_id VARCHAR(255),
    old_value TEXT,
    new_value TEXT,
    ip_address INET,
    user_agent TEXT,
    request_id VARCHAR(100),  -- For tracing related logs
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_admin_audit_logs_action ON admin_audit_logs(action);
CREATE INDEX idx_admin_audit_logs_admin_user ON admin_audit_logs(admin_user);
CREATE INDEX idx_admin_audit_logs_target ON admin_audit_logs(target_type, target_id);
CREATE INDEX idx_admin_audit_logs_created_at ON admin_audit_logs(created_at DESC);

-- ============================================================================
-- Worker Status Cache Table (for faster worker monitoring)
-- ============================================================================
CREATE TABLE IF NOT EXISTS worker_status_cache (
    id SERIAL PRIMARY KEY,
    miner_address VARCHAR(255) NOT NULL,
    worker_name VARCHAR(255) NOT NULL,
    last_seen TIMESTAMPTZ DEFAULT NOW(),
    is_online BOOLEAN DEFAULT false,
    current_hashrate BIGINT DEFAULT 0,
    current_difficulty BIGINT DEFAULT 0,
    total_shares BIGINT DEFAULT 0,
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT unique_worker UNIQUE (miner_address, worker_name)
);

-- Indexes
CREATE INDEX idx_worker_status_miner ON worker_status_cache(miner_address);
CREATE INDEX idx_worker_status_is_online ON worker_status_cache(is_online);
CREATE INDEX idx_worker_status_last_seen ON worker_status_cache(last_seen DESC);

-- ============================================================================
-- Block Details Cache Table (for faster block queries)
-- ============================================================================
CREATE TABLE IF NOT EXISTS block_details_cache (
    block_height INTEGER PRIMARY KEY,
    block_hash VARCHAR(255) NOT NULL,
    block_time TIMESTAMPTZ NOT NULL,
    reward_sats BIGINT NOT NULL,
    fee_sats BIGINT DEFAULT 0,
    pool_fee_sats BIGINT NOT NULL,
    pplns_window_shares INTEGER NOT NULL,
    pplns_total_difficulty BIGINT NOT NULL,
    payout_count INTEGER NOT NULL,
    coinbase_txid VARCHAR(255),
    confirmations INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index
CREATE INDEX idx_block_details_cache_time ON block_details_cache(block_time DESC);

-- ============================================================================
-- Functions and Triggers
-- ============================================================================

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Apply the trigger to relevant tables
CREATE TRIGGER update_banned_miners_updated_at BEFORE UPDATE ON banned_miners
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_custom_thresholds_updated_at BEFORE UPDATE ON custom_thresholds
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_notification_configs_updated_at BEFORE UPDATE ON notification_configs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_system_configs_updated_at BEFORE UPDATE ON system_configs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_worker_status_updated_at BEFORE UPDATE ON worker_status_cache
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_block_details_updated_at BEFORE UPDATE ON block_details_cache
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- Views for Common Queries
-- ============================================================================

-- View: Active miners with hashrate (last 24h)
CREATE OR REPLACE VIEW active_miners_24h AS
SELECT
    m.address,
    m.balance_sats,
    COALESCE(SUM(DISTINCT s.difficulty), 0) as total_shares_24h,
    COUNT(DISTINCT w.worker_name) as worker_count,
    MAX(w.last_seen) as last_seen
FROM miners m
LEFT JOIN shares s ON s.miner_id = m.id
    AND s.created_at > NOW() - INTERVAL '24 hours'
LEFT JOIN worker_status_cache w ON w.miner_address = m.address
GROUP BY m.id, m.address, m.balance_sats
ORDER BY total_shares_24h DESC;

-- View: Miners pending payout
CREATE OR REPLACE VIEW miners_pending_payout AS
SELECT
    m.address,
    m.balance_sats,
    COALESCE(ct.threshold_sats, sc.value::BIGINT) as threshold_sats,
    m.balance_sats >= COALESCE(ct.threshold_sats, sc.value::BIGINT) as above_threshold
FROM miners m
LEFT JOIN custom_thresholds ct ON ct.address = m.address
CROSS JOIN (SELECT value FROM system_configs WHERE key = 'pool.min_payout_sats') sc
WHERE m.balance_sats > 0;

-- ============================================================================
-- Grant permissions (adjust as needed)
-- ============================================================================
-- GRANT SELECT, INSERT, UPDATE ON ALL TABLES IN SCHEMA public TO dmpool_app;
-- GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO dmpool_app;

-- Migration complete
SELECT 'Migration 001 completed successfully' as status;
