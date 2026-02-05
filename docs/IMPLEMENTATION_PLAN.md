# DMPool 完整实施计划

**基于**: DMPool_Decentralized_AI_Solution_v2.md
**创建时间**: 2026-02-05
**目标**: 完整实现 PRD 第三章定义的所有核心功能

---

## 📊 项目概览

### 技术栈分工

| Agent | 模块 | 技术栈 | 状态 |
|-------|------|--------|------|
| Agent-1 | 观察者前端 | React 19 + Vite + TailwindCSS + Recharts | ❌ 未开始 |
| Agent-2 | 观察者 API | Rust + Axum | ⚠️ 10% (依赖 p2poolv2_api) |
| Agent-3 | 管理后台前端 | Vue 3 + Vben Admin | ⚠️ 30% (基础框架) |
| Agent-4 | 管理后台 API | Rust + Axum | ⚠️ 20% (基础功能) |
| Agent-5 | 基础设施 | Docker + Nginx + PostgreSQL | ✅ 80% (配置完成) |

---

## 🗄️ 数据库 Schema 设计

### 新增表（管理功能）

```sql
-- 黑名单矿工表
CREATE TABLE banned_miners (
    id SERIAL PRIMARY KEY,
    address VARCHAR(255) UNIQUE NOT NULL,
    banned_at TIMESTAMPTZ DEFAULT NOW(),
    banned_by VARCHAR(255), -- 管理员
    reason TEXT,
    INDEX idx_address (address)
);

-- 自定义支付阈值表
CREATE TABLE custom_thresholds (
    address VARCHAR(255) PRIMARY KEY,
    threshold_sats BIGINT NOT NULL DEFAULT 1000000, -- 0.01 BTC
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    updated_by VARCHAR(255),
    INDEX idx_address (address)
);

-- 通知配置表
CREATE TABLE notification_configs (
    id SERIAL PRIMARY KEY,
    user_type VARCHAR(50) NOT NULL, -- 'admin' or 'miner'
    address VARCHAR(255), -- 矿工地址 (admin 可为 null)
    telegram_enabled BOOLEAN DEFAULT false,
    telegram_chat_id VARCHAR(255),
    email_enabled BOOLEAN DEFAULT false,
    email_address VARCHAR(255),
    notify_block_found BOOLEAN DEFAULT true,
    notify_payment_received BOOLEAN DEFAULT true,
    notify_system_alert BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    INDEX idx_user_type (user_type),
    INDEX idx_address (address)
);

-- 通知历史表
CREATE TABLE notification_history (
    id SERIAL PRIMARY KEY,
    config_id INTEGER REFERENCES notification_configs(id),
    notification_type VARCHAR(50) NOT NULL, -- 'block_found', 'payment', 'alert'
    channel VARCHAR(20) NOT NULL, -- 'telegram', 'email'
    content TEXT,
    status VARCHAR(20) NOT NULL, -- 'pending', 'sent', 'failed'
    sent_at TIMESTAMPTZ,
    error_message TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    INDEX idx_type (notification_type),
    INDEX idx_status (status)
);

-- 系统配置表（动态配置）
CREATE TABLE system_configs (
    key VARCHAR(100) PRIMARY KEY,
    value TEXT NOT NULL,
    value_type VARCHAR(20) NOT NULL, -- 'string', 'number', 'boolean', 'json'
    description TEXT,
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    updated_by VARCHAR(255)
);

-- 操作日志表（扩展）
CREATE TABLE admin_audit_logs (
    id SERIAL PRIMARY KEY,
    admin_user VARCHAR(255) NOT NULL,
    action VARCHAR(100) NOT NULL, -- 'ban_miner', 'update_threshold', 'manual_payout', etc.
    target_type VARCHAR(50), -- 'miner', 'worker', 'config', etc.
    target_id VARCHAR(255),
    old_value TEXT,
    new_value TEXT,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    INDEX idx_action (action),
    INDEX idx_admin (admin_user),
    INDEX idx_created (created_at)
);
```

### Hydrapool 原有表（只读访问）

```sql
-- 已有的表（Hydrapool 创建）
TABLE shares (
    id BIGSERIAL PRIMARY KEY,
    miner_id INTEGER,
    difficulty BIGINT,
    created_at TIMESTAMPTZ,
    -- ... 其他字段
)

TABLE miners (
    id SERIAL PRIMARY KEY,
    address VARCHAR(255) UNIQUE NOT NULL,
    balance_sats BIGINT DEFAULT 0,
    -- ... 其他字段
)

TABLE payouts (
    id SERIAL PRIMARY KEY,
    miner_id INTEGER,
    amount_sats BIGINT,
    txid VARCHAR(255),
    -- ... 其他字段
)
```

---

## 🔌 API 接口定义

### 观察者 API（公开访问）

#### 1. 矿池全局统计
```
GET /api/v1/stats

Response:
{
  "pool_hashrate_3h": 321500000000000,
  "active_miners": 156,
  "active_workers": 342,
  "last_block_height": 823456,
  "next_block_eta_seconds": 3600,
  "pool_fee_percent": 1.0,
  "network_difficulty": 71234567890000,
  "block_reward": 3.125
}
```

#### 2. 矿工完整数据
```
GET /api/v1/stats/{address}

Response:
{
  "address": "bc1q...",
  "shares_in_window": 36710000000,
  "estimated_reward_window": 0.00076836,
  "estimated_next_block": 0.00009445,
  "hashrate_3h": 321500000000000,
  "hashrate_avg": {
    "1h": 315200000000000,
    "6h": 308700000000000,
    "24h": 295300000000000,
    "7d": 280100000000000
  },
  "workers": [
    {
      "name": "rig01",
      "hashrate": 156200000000000,
      "shares": 2300000,
      "last_seen": "2026-02-05T10:23:45Z",
      "is_online": true
    }
  ],
  "latest_earnings": [
    {
      "block_height": 823129,
      "time": "2026-02-05T08:15:32Z",
      "amount_btc": 0.00543,
      "txid": "abc123...",
      "confirmations": 100
    }
  ]
}
```

#### 3. 算力历史数据
```
GET /api/v1/stats/{address}/hashrate?period=7d

Response:
{
  "address": "bc1q...",
  "period": "7d",
  "interval": "1h",
  "data_points": [
    { "timestamp": "2026-02-05T00:00:00Z", "hashrate": 320000000000000 },
    { "timestamp": "2026-02-05T01:00:00Z", "hashrate": 318000000000000 },
    // ...
  ]
}
```

#### 4. 区块历史
```
GET /api/v1/blocks?limit=20&offset=0

Response:
{
  "total": 156,
  "blocks": [
    {
      "height": 823456,
      "time": "2026-02-05T10:00:00Z",
      "reward_btc": 3.125,
      "pool_fee_percent": 1.0,
      "txid": "abc123...",
      "confirmations": 10,
      "payouts_count": 87
    }
  ]
}
```

#### 5. 区块详情
```
GET /api/v1/blocks/{height}

Response:
{
  "height": 823456,
  "time": "2026-02-05T10:00:00Z",
  "reward_btc": 3.125,
  "pool_fee_btc": 0.03125,
  "network_difficulty": 71234567890000,
  "txid": "abc123...",
  "confirmations": 10,
  "pplns_window_shares": 10000,
  "payouts": [
    {
      "address": "bc1q...",
      "amount_btc": 0.00543,
      "shares": 234000,
      "share_percent": 2.34
    }
  ]
}
```

### 管理后台 API（内网访问，需认证）

#### 1. 仪表盘数据
```
GET /api/admin/dashboard

Response:
{
  "pool": {
    "hashrate_24h": 315000000000000,
    "active_miners": 156,
    "active_workers": 342,
    "shares_per_second": 1234
  },
  "blocks": {
    "last_found": "2026-02-05T08:15:32Z",
    "last_height": 823456,
    "total_found": 156
  },
  "payments": {
    "pending_amount_btc": 1.234,
    "pending_count": 23,
    "last_paid": "2026-02-05T06:00:00Z"
  },
  "system": {
    "stratum_connections": 342,
    "api_requests_per_minute": 45,
    "db_connections": 5,
    "uptime_seconds": 86400
  }
}
```

#### 2. 矿工列表
```
GET /api/admin/miners?limit=20&offset=0&search=bc1q

Response:
{
  "total": 156,
  "miners": [
    {
      "id": 1,
      "address": "bc1q...",
      "hashrate_24h": 150000000000000,
      "balance_btc": 0.0234,
      "total_earned_btc": 1.234,
      "workers_count": 3,
      "last_seen": "2026-02-05T10:23:45Z",
      "is_banned": false,
      "custom_threshold_btc": 0.01
    }
  ]
}
```

#### 3. 矿工详情
```
GET /api/admin/miners/{address}

Response:
{
  "address": "bc1q...",
  "hashrate_24h": 150000000000000,
  "hashrate_avg": { /* 同观察者 API */ },
  "balance_btc": 0.0234,
  "total_earned_btc": 1.234,
  "total_paid_btc": 1.2106,
  "workers": [ /* Worker 详情 */ ],
  "latest_shares": [ /* 最近份额 */ ],
  "custom_threshold_btc": 0.01
}
```

#### 4. 禁用/启用矿工
```
POST /api/admin/miners/{address}/ban
Body: { "reason": "在攻击矿池", "permanent": false }

Response: { "success": true }

DELETE /api/admin/miners/{address}/ban

Response: { "success": true }
```

#### 5. 修改支付阈值
```
PUT /api/admin/miners/{address}/threshold
Body: { "threshold_btc": 0.05 }

Response: { "success": true, "new_threshold_btc": 0.05 }
```

#### 6. 工作者列表
```
GET /api/admin/workers?limit=50&status=online

Response:
{
  "total": 342,
  "workers": [
    {
      "id": 1,
      "miner_address": "bc1q...",
      "name": "rig01",
      "hashrate": 156200000000000,
      "difficulty": 5000,
      "shares": 2300000,
      "last_seen": "2026-02-05T10:23:45Z",
      "is_online": true
    }
  ]
}
```

#### 7. 支付管理
```
GET /api/admin/payments/pending

Response:
{
  "total_btc": 1.234,
  "count": 23,
  "payments": [
    {
      "address": "bc1q...",
      "balance_btc": 0.0234,
      "threshold_btc": 0.01,
      "unpaid_since": "2026-02-03T10:00:00Z"
    }
  ]
}

POST /api/admin/payments/trigger/{address}
Body: { "amount_btc": 0.02 }

Response: { "success": true, "txid": "abc123..." }

GET /api/admin/payments/history?limit=20

Response:
{
  "total": 156,
  "payments": [
    {
      "id": 1,
      "address": "bc1q...",
      "amount_btc": 0.00543,
      "txid": "abc123...",
      "block_height": 823129,
      "confirmations": 100,
      "status": "confirmed",
      "created_at": "2026-02-05T08:15:32Z"
    }
  ]
}
```

#### 8. 区块管理
```
GET /api/admin/blocks?limit=20

Response:
{
  "total": 156,
  "blocks": [ /* 同观察者 API，增加 PPLNS 详情 */ ]
}

GET /api/admin/blocks/{height}/pplns

Response:
{
  "height": 823456,
  "pplns_window_shares": 10000,
  "total_difficulty": 50000000000,
  "payouts": [
    {
      "address": "bc1q...",
      "difficulty": 1170000000,
      "share_percent": 2.34,
      "reward_btc": 0.07312
    }
  ]
}
```

#### 9. 系统监控
```
GET /api/admin/monitoring/stratum

Response:
{
  "connections": 342,
  "unique_ips": 89,
  "shares_per_second": 1234,
  "average_difficulty": 4500
}

GET /api/admin/monitoring/database

Response:
{
  "connections": 5,
  "database_size_mb": 1234,
  "shares_count": 12345678,
  "avg_query_time_ms": 5
}

GET /api/admin/logs?level=error&limit=50

Response:
{
  "logs": [
    {
      "timestamp": "2026-02-05T10:23:45Z",
      "level": "error",
      "message": "Failed to submit share",
      "context": { "miner": "bc1q...", "error": "..." }
    }
  ]
}
```

#### 10. 通知配置
```
GET /api/admin/notifications/config

Response:
{
  "admin_telegram_enabled": true,
  "admin_email_enabled": true,
  "admin_telegram_chat_id": "...",
  "admin_email_address": "admin@dmpool.org"
}

PUT /api/admin/notifications/config
Body: {
  "admin_telegram_enabled": true,
  "admin_email_enabled": true,
  "notify_block_found": true,
  "notify_payment": true,
  "notify_alert": true
}

Response: { "success": true }

GET /api/admin/notifications/history?limit=20

Response:
{
  "total": 1234,
  "notifications": [ /* 通知历史 */ ]
}
```

#### 11. 系统配置
```
GET /api/admin/config

Response:
{
  "pool_fee_percent": 1.0,
  "min_payout_btc": 0.01,
  "pplns_window_days": 7,
  "stratum_port": 3333,
  "api_port": 8081
}

PUT /api/admin/config
Body: {
  "pool_fee_percent": 1.5,
  "min_payout_btc": 0.005
}

Response: { "success": true, "reload_required": true }
```

---

## 🎯 分阶段实施计划

### Phase 1: 基础设施 + 数据库 (3-5 天)
- [ ] 创建数据库 Schema
- [ ] 编写数据库迁移脚本
- [ ] 设置 Docker Compose 环境
- [ ] 验证 Hydrapool 连接

### Phase 2: 观察者 API (5-7 天)
- [ ] 实现 `/api/v1/stats` 端点
- [ ] 实现 `/api/v1/stats/{address}` 端点
- [ ] 实现 `/api/v1/stats/{address}/hashrate` 端点
- [ ] 实现 `/api/v1/blocks` 端点
- [ ] 实现 `/api/v1/blocks/{height}` 端点
- [ ] 单元测试

### Phase 3: 管理后台 API (7-10 天)
- [ ] 实现仪表盘端点
- [ ] 实现矿工管理端点
- [ ] 实现工作者监控端点
- [ ] 实现支付管理端点
- [ ] 实现区块管理端点
- [ ] 实现系统监控端点
- [ ] 实现通知配置端点
- [ ] 实现系统配置端点

### Phase 4: 观察者前端 (7-10 天)
- [ ] 初始化 React + Vite 项目
- [ ] 实现矿工搜索页面
- [ ] 实现观察者页面（仿 OCEAN）
- [ ] 集成 Recharts 算力图表
- [ ] 实现 Workers 表格
- [ ] 实现收益记录表格
- [ ] 响应式设计

### Phase 5: 管理后台前端 (7-10 天)
- [ ] 初始化 Vue 3 + Vben Admin
- [ ] 实现仪表盘
- [ ] 实现矿工管理
- [ ] 实现工作者监控
- [ ] 实现支付管理
- [ ] 实现区块管理
- [ ] 实现系统监控
- [ ] 实现通知配置
- [ ] 实现系统配置

### Phase 6: 通知系统 (3-5 天)
- [ ] Telegram Bot 集成
- [ ] Email 发送服务
- [ ] 事件触发机制
- [ ] 通知模板

### Phase 7: 测试与部署 (3-5 天)
- [ ] 集成测试
- [ ] 性能测试
- [ ] 安全测试
- [ ] 部署到 homelab
- [ ] 文档完善

**总计：约 35-52 天（约 1-2 个月）**

---

## 📁 项目结构

```
dmpool-rust/
├── src/
│   ├── main.rs              # 主服务入口
│   ├── lib.rs               # 库入口
│   ├── observer_api/        # 观察者 API (新建)
│   │   ├── mod.rs
│   │   ├── routes/
│   │   │   ├── stats.rs
│   │   │   ├── blocks.rs
│   │   │   └── mod.rs
│   │   ├── models/
│   │   ├── db/
│   │   └── middleware/
│   ├── admin_api/           # 管理后台 API (新建)
│   │   ├── mod.rs
│   │   ├── routes/
│   │   │   ├── dashboard.rs
│   │   │   ├── miners.rs
│   │   │   ├── workers.rs
│   │   │   ├── payments.rs
│   │   │   ├── blocks.rs
│   │   │   ├── monitoring.rs
│   │   │   ├── notifications.rs
│   │   │   ├── config.rs
│   │   │   └── mod.rs
│   │   ├── models/
│   │   ├── db/
│   │   └── middleware/
│   ├── notification/        # 通知系统 (新建)
│   │   ├── mod.rs
│   │   ├── telegram.rs
│   │   ├── email.rs
│   │   └── templates.rs
│   ├── config/              # 动态配置 (新建)
│   │   ├── mod.rs
│   │   └── storage.rs
│   ├── db/                  # 数据库模块 (新建)
│   │   ├── mod.rs
│   │   ├── connection.rs
│   │   ├── schema.rs
│   │   └── queries.rs
│   └── ...                  # 现有模块
├── web-observer/            # 观察者前端 (新建)
│   ├── src/
│   │   ├── pages/
│   │   │   ├── Home.tsx
│   │   │   ├── Search.tsx
│   │   │   └── Observer.tsx
│   │   ├── components/
│   │   │   ├── HashrateChart.tsx
│   │   │   ├── WorkersTable.tsx
│   │   │   └── EarningsTable.tsx
│   │   ├── api/
│   │   ├── hooks/
│   │   └── utils/
│   ├── package.json
│   └── vite.config.ts
├── web-admin/               # 管理后台前端 (新建)
│   ├── src/
│   │   ├── views/
│   │   │   ├── Dashboard.vue
│   │   │   ├── Miners.vue
│   │   │   ├── Workers.vue
│   │   │   ├── Payments.vue
│   │   │   ├── Blocks.vue
│   │   │   ├── Monitoring.vue
│   │   │   ├── Notifications.vue
│   │   │   └── Settings.vue
│   │   ├── components/
│   │   ├── api/
│   │   └── router/
│   ├── package.json
│   └── vite.config.ts
├── migrations/              # 数据库迁移 (新建)
│   ├── 001_initial_schema.sql
│   ├── 002_admin_tables.sql
│   └── 003_notification_tables.sql
├── docker/
│   ├── Dockerfile
│   ├── Dockerfile.admin
│   ├── nginx.conf
│   └── init.sql
├── docker-compose.yml
├── config.toml
└── docs/
    ├── API.md               # API 文档
    ├── DEPLOYMENT.md
    └── PRODUCTION_STATUS.md
```

---

## ⚠️ 关键依赖

1. **p2poolv2_api**: 只提供了基础端点，需要扩展或新建独立的 Observer API
2. **Hydrapool**: 提供核心功能，需要通过数据库读取数据
3. **PostgreSQL**: 需要连接到 Hydrapool 的数据库

---

**下一步**: 开始 Phase 1 - 数据库 Schema 创建
