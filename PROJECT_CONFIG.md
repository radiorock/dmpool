# DMPool 项目配置与状态

## 项目信息

- **项目名称**: DMPool
- **性质**: 商业矿池项目（保密）
- **基于**: Hydrapool (256-Foundation)
- **许可**: AGPLv3

## ⚠️ 重要提醒

**这是一个商业项目！请遵守以下规则：**

1. **不要发布到公开 GitHub 仓库** - 这是内部项目，不需要那么多人知道
2. **保密开发** - 技术细节和配置不应对外公开
3. **文档安全** - 敏感配置信息不要提交到任何公开位置

## 已完成功能

### ✅ 任务 B: 观察者链接功能

**状态**: 已完成

**实现内容**:
1. 添加公开观察者 API 端点 `/api/observer/:address`
2. 创建观察者搜索页面 `/observer`
3. 创建观察者统计页面 `/observer/:address`
4. 采用明亮简洁的 UI 设计（区别于 ocean.xyz 的暗色风格）

**文件变更**:
- `src/bin/dmpool_admin.rs`: 添加观察者 API 处理函数
- `static/admin/observer.html`: 观察者前端页面
- `docs/OBSERVER_GUIDE.md`: 观察者功能文档

**API 示例**:
```bash
curl http://localhost:8080/api/observer/bc1q...
```

**访问示例**:
```
http://localhost:8080/observer/bc1q...
```

### ✅ 任务 D: 文档编写

**状态**: 已完成

**已创建文档**:
1. `docs/API.md` - API 文档
2. `docs/DEVELOPMENT.md` - 开发指南
3. `docs/DEPLOYMENT.md` - 部署文档
4. `docs/OBSERVER_GUIDE.md` - 观察者功能指南
5. `docs/PRD_MVP_2026-02-04.md` - PRD 文档
6. `BUSINESS_LOGIC_AUDIT.md` - 业务逻辑审计
7. `SECURITY_AUDIT.md` - 安全审计

### 🔄 任务 C: 前端增强

**状态**: 待进行

**计划内容**:
1. 改进管理后台 dashboard
2. 添加更多可视化图表
3. 优化移动端体验

## 部署状态

### 服务端口

| 服务 | 端口 | 说明 |
|------|------|------|
| dmpool (主矿池) | 3333 | Stratum 端口 |
| dmpool (API) | 8081 | 公开 API |
| dmpool-admin | 8080 | 管理后台 + 观察者页面 |

### 环境变量

```bash
# Admin 服务
CONFIG_PATH=config/homelab-signet.toml
ADMIN_USERNAME=admin
ADMIN_PASSWORD=DMPool@Admin2026
JWT_SECRET=<your-secret-key>
```

## 技术栈

- **后端**: Rust (p2poolv2 v0.7.0)
- **前端**: HTML + Tailwind CSS + Chart.js
- **数据库**: RocksDB
- **监控**: Prometheus + Grafana

## 开发规范

1. **不要修改核心代码** - p2poolv2 库保持原样，便于上游兼容
2. **仅扩展管理功能** - 专注于管理后台和前端页面
3. **测试后更新文档** - 完成功能后及时更新文档

## Git 分支

- `master` - 主分支
- `admin-api` - 管理后台和观察者功能开发分支

## 下一步工作

1. 恢复 homelab SSH 连接
2. 部署观察者功能
3. 测试观察者链接
4. 开始前端增强工作

---

**更新日期**: 2026-02-04
**更新人**: K4y
