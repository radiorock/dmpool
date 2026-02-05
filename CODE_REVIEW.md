# DMPool 代码审查报告

**审查日期**: 2026-02-05
**审查范围**: 后端Rust代码 + 前端Vue/React代码
**审查者**: Codex (后端) + Claude (前端 + 综合)

---

## 📋 审查统计

- **后端文件**: Rust (src/admin_api/, src/observer_api/, src/auth/, src/db/, src/payment/)
- **前端文件**: Vue 3 (web-admin/) + React 19 (web-observer/)
- **代码行数**: 约8000+ 行后端代码
- **部署状态**: 前端已部署到生产环境，后端API运行中但有问题

---

## 🚨 关键问题 (Critical) - 必须修复

### 1. ❌ **Admin API完全未认证**
**位置**: `src/admin_api/mod.rs:33-75`
```rust
pub fn create_router(db: Arc<DatabaseManager>) -> Router {
    let state = AdminState { db };
    Router::new()
        .route("/api/admin/dashboard", get(routes::dashboard::get_dashboard))
        // ... 所有admin端点都没有认证中间件！
```

**问题**：
- Admin API路由器**从未应用认证中间件**
- 所有关键端点（矿工管理、支付、系统配置）完全公开
- 尽管注释说"仅限VPN/内网访问"，但代码层面无任何保护

**风险**：任何人都可以访问 `/api/admin/dashboard`、`/api/admin/miners` 等敏感端点

**修复**：
```rust
.use_layer(axum::middleware::from_fn(|req, next| {
    // 检查VPN IP或JWT token
    next.allow()
}))
```

---

### 2. ❌ **SQL注入漏洞**
**位置**: `src/admin_api/routes/payments.rs:216-260`

```rust
// 直接字符串拼接SQL！
if let Some(address) = &query.address {
    conditions.push(format!("address = '{}'", address));
}

if let Some(status) = &query.status {
    conditions.push(format!("status = '{}'", status));
}

// 然后直接执行这个拼接的SQL！
let rows = conn.query(&sql, &[]).await?;
```

**问题**：
- 用户输入直接拼接到SQL字符串
- 参数 `address` 和 `status` 完全未经过验证或转义
- 攻击示例：`status=' OR '1'='1` 可以绕过过滤
- 更严重的：`status='; DROP TABLE payouts;--` 可以删除数据

**风险**：数据库被攻击、数据泄露、数据破坏

**修复**：使用参数化查询
```rust
let mut query = "SELECT id, address, ... FROM payout_history_view WHERE 1=1";
let mut param_count = 0;
let mut conditions = Vec::new();

if let Some(address) = &query.address {
    conditions.push(format!("${} = ${}", param_count + 1));
    // ... 使用参数化
}
```

---

### 3. ❌ **编译错误 + SQL注入**
**位置**: `src/admin_api/routes/miners.rs:65-76`

```rust
// 引用了不存在的变量！
let rows = conn.query(&sql, &[&search]).await?;
//                              ^^^^^^ undefined variable 'search'
```

**问题**：
- 代码根本无法编译（`search` 变量未定义）
- 即使修复编译错误，SQL注入仍然存在

**风险**：代码无法运行，且即使修复也有安全漏洞

---

## ⚠️ 主要问题 (Major)

### 4. **数据库连接字符串泄露**
**位置**: `src/db/mod.rs:22-45`

```rust
info!("Connecting to database: {}", conn_string);
```

**问题**：
- 完整的数据库连接字符串（包含用户名、密码、主机）被记录到日志
- 日志可能被发送到监控系统、日志聚合器

**风险**：数据库凭据泄露

**修复**：
```rust
info!("Connecting to database: postgresql://***@{}",
    conn_string.split('@').last().unwrap_or("unknown"));
```

---

### 5. **支付计算下溢出**
**位置**: `src/payment/mod.rs:335-370`

```rust
let change_satoshis = total_input - payout.amount_satoshis;
// 如果 total_input < payout.amount_satoshis 会怎样？
// unsigned subtraction 会下溢出！
```

**问题**：
- 未检查第一个UTXO是否足够支付
- `saturating_sub` 只解决了部分问题
- 如果金额不足会创建错误的交易

**风险**：支付失败、资金锁定

---

### 6. **参数化SQL占位符错误**
**位置**: `src/admin_api/routes/payments.rs:87-101`

```rust
let sql = "... $1, $2, $3 ...";
conn.query(&sql, &[])  // 空参数数组！
```

**问题**：
- SQL中有 `$1, $2, $3` 占位符
- 但传递的是空数组 `&[]`
- 导致查询失败："no parameter $1"

**影响**：管理员无法查看待支付列表

---

## 🟡 次要问题 (Minor)

### 7. **找零地址回退**
**位置**: `src/payment/mod.rs:378-389`

当RPC不返回地址时使用交易ID作为地址，会产生无效交易。

### 8. **比特币地址验证不完整**
**位置**: `src/observer_api/routes/mod.rs:145-150`

只检查前缀（`bc1/1/3`），不验证校验和，无效地址也会查询数据库。

---

## 🎯 前端问题（检查结果）

### ✅ 做得好的地方

1. **JWT Token存储** - 使用 `localStorage`
2. **Bearer认证** - 正确使用 `Authorization: Bearer ${token}`
3. **TypeScript严格模式** - `web-admin` 使用类型
4. **API错误处理** - 有基本的try-catch
5. **无console.log残留** - `web-observer` 很干净

### ⚠️ 发现的问题

1. **硬编码API地址**
   ```typescript
   const API_BASE_URL = import.meta.env.VITE_ADMIN_API_URL || 'http://localhost:8080/admin';
   ```
   - 生产环境应该配置正确的前端API路径

2. **无CORS配置**
   - Admin API和Observer API可能需要CORS头

3. **敏感数据暴露**
   - localStorage存储的JWT token可被XSS读取
   - 建议使用HttpOnly cookie

---

## 📊 总体评分

**综合得分**: **3.5/10** ⭐⭐☆☆☆

### 评分细分

| 类别 | 得分 | 说明 |
|------|------|------|
| 安全性 | 2/10 | ❌ SQL注入、未认证Admin API、凭据泄露 |
| 性能 | 6/10 | ⚠️ 基本的连接池，但有参数化错误 |
| 代码质量 | 4/10 | ❌ 编译错误、日志泄露、下溢出风险 |
| 前端质量 | 7/10 | ✅ 类型安全、⚠️ API配置问题 |
| 可维护性 | 5/10 | ⚠️ 缺乏文档、测试覆盖不足 |

---

## ✅ 生产就绪度

**结论**: ❌ **不适合生产环境**

### 必须修复才能上线

1. **修复Admin API认证** - Critical
2. **修复所有SQL注入** - Critical
3. **修复编译错误** - Critical
4. **添加输入验证** - Major
5. **修复支付计算** - Major

### 建议修复优先级

**P0 (立即修复)**:
1. Admin API添加认证中间件
2. 修复SQL注入漏洞
3. 修复参数化查询错误
4. 修复编译错误

**P1 (尽快修复)**:
5. 移除敏感日志
6. 修复支付计算下溢出
7. 添加输入验证

**P2 (后续优化)**:
8. 改进地址验证
9. 添加单元测试
10. 配置CORS和安全头

---

## 🔧 快速修复建议

### 1. Admin API认证（5分钟）

```rust
// src/admin_api/mod.rs
use crate::auth::require_auth;

pub fn create_router(db: Arc<DatabaseManager>) -> Router {
    Router::new()
        .route("/api/admin/dashboard", get(routes::dashboard::get_dashboard))
        .layer(axum::middleware::from_fn(
            crate::admin_api::middleware::auth_middleware
        ))
}
```

### 2. SQL注入修复（10分钟）

使用参数化查询或白名单验证：
```rust
// 验证address格式
if !bitcoin_address::is_valid(&address) {
    return Err(AdminError::InvalidAddress);
}

// 参数化查询
conn.query(
    "SELECT * FROM payout_history_view WHERE address = $1 AND status = $2",
    &[&address, &status]
)
```

### 3. 编译错误修复（2分钟）

```rust
// src/admin_api/routes/miners.rs:68
let rows = if let Some(ref search) = query.search {
    conn.query(&sql, &[search, &limit, &offset]).await?
} else {
    conn.query(&sql, &[&limit, &offset]).await?
};
```

---

## 📌 后续行动

1. ⛔ **暂停生产使用** - 当前代码不安全
2. 🔧 **修复所有Critical问题**
3. ✅ **通过安全审查**
4. 🚀 **重新部署**

**重要**: 在修复所有Critical问题之前，**强烈建议不要将此系统用于真实挖矿**，因为：
- Admin面板无保护
- SQL注入可清空数据库
- 支付计算可能失败

---

**审查者签名**: Codex + Claude
**审查工具**: codeagent-wrapper
**报告生成**: 2026-02-05 18:15
