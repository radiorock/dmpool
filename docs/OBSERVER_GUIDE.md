# DMPool 观察者链接功能

## 概述

观察者链接功能允许矿工通过公开链接查看自己的挖矿统计数据，无需登录管理后台。这是矿池的核心功能之一，让矿工可以透明地查看自己的贡献和收益。

**注意：这是商业项目的内部文档，请勿对外公开。**

## 功能特性

- **公开访问**：无需认证即可查看
- **实时数据**：每30秒自动刷新
- **简洁设计**：采用明亮、现代的 UI 设计，区别于传统的暗色矿池界面
- **响应式布局**：支持桌面和移动设备
- **多 Worker 支持**：显示每个矿工机器的详细统计

## API 端点

### 1. 获取矿工统计数据

```http
GET /api/observer/:address
```

**路径参数：**
- `address`: Bitcoin 地址（bc1q... 格式）

**响应示例：**
```json
{
  "status": "ok",
  "data": {
    "address": "bc1q...",
    "total_shares": 12345,
    "total_difficulty": 12345000,
    "workers": {
      "worker1": {
        "name": "worker1",
        "shares": 6234,
        "difficulty": 6172500,
        "first_seen": 1738564800,
        "last_seen": 1738594800
      }
    },
    "hashrate_ths": 123.45,
    "last_share": 1738594800,
    "first_share": 1738564800,
    "period_hours": 24
  }
}
```

### 2. 观察者页面

```http
GET /observer/:address
```

返回用于显示统计数据的 HTML 页面。

### 3. 搜索页面

```http
GET /observer
```

返回搜索页面，允许用户输入 Bitcoin 地址进行查询。

## 数据说明

### 估计算力计算

算力估算基于过去 24 小时的 shares：

```
Hashrate (TH/s) = (Total Difficulty * 2^32) / (Time Window in seconds * 10^12)
```

这是一个粗略估计，实际算力可能因网络延迟、难度调整等因素有所不同。

### Workers 状态

- **在线**：在数据周期内（24小时）有提交 Share
- **离线**：在数据周期内无提交

## 前端设计

### 设计原则

参考了 ocean.xyz 的功能，但采用了完全不同的设计风格：

- **配色方案**：
  - 背景：浅灰色 (#fafafa)
  - 卡片：纯白色 (#ffffff)
  - 强调色：蓝紫渐变 (#3b82f6 → #8b5cf6)

- **字体**：
  - 正文：Inter
  - 代码/地址：JetBrains Mono

- **交互**：
  - 悬停效果：卡片轻微上浮
  - 加载状态：旋转加载动画
  - 在线状态：脉动绿点

### 响应式断点

- 桌面：≥1024px（4列网格）
- 平板：768px - 1023px（2列网格）
- 移动：<768px（单列）

## 部署

观察者链接功能集成在 `dmpool_admin` 服务中，默认监听端口 8080。

### 环境变量

```bash
CONFIG_PATH=config/homelab-signet.toml
ADMIN_USERNAME=admin
ADMIN_PASSWORD=DMPool@Admin2026  # 必须满足密码策略
```

### 启动服务

```bash
/home/k0n9/dmpool/dmpool-rust/target/release/dmpool_admin
```

## 使用示例

### 1. 访问搜索页面

```
http://pool.example.com:8080/observer
```

### 2. 直接查看特定地址

```
http://pool.example.com:8080/observer/bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh
```

### 3. API 调用

```bash
curl http://pool.example.com:8080/api/observer/bc1q...
```

## 性能考虑

- API 查询限制在过去 24 小时的 5000 条 shares
- 数据库查询经过索引优化
- 前端每 30 秒自动刷新一次

## 未来改进

- [ ] 添加历史算力图表
- [ ] 支持自定义时间范围
- [ ] 添加收益估算
- [ ] 支持 PPLNS 窗口可视化
- [ ] 添加分享按钮（生成可分享链接）

## 相关文档

- [API 文档](./API.md)
- [开发指南](./DEVELOPMENT_GUIDE_CN.md)
- [部署文档](./DEPLOYMENT.md)

---

**机密声明：本文档为 DMPool 商业项目内部文档，包含敏感信息。请勿对外分享或发布到公开仓库。**
