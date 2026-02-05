# DMPool 部署指南

**版本**: v2.0
**更新时间**: 2026-02-05
**基于**: DMPool_Decentralized_AI_Solution_v2.md

---

## 部署架构

```
┌─────────────────────────────────────────────────────────────┐
│                    Internet                                  │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│   Cloudflare Pages (www.dmpool.org)                        │
│   - Observer Frontend (React + Vite)                        │
│   - Static Assets                                           │
└─────────────────────────────────────────────────────────────┘
                           │
                           │ API 请求 (只读)
                           ▼
┌─────────────────────────────────────────────────────────────┐
│   Nginx (反向代理)                                          │
│   - SSL Termination                                         │
│   - Rate Limiting                                           │
│   - API 路径映射 (/api/v1/stats/*)                         │
│   - 网络隔离 (管理后台仅内网)                               │
└─────────────────────────────────────────────────────────────┘
                           │
        ┌──────────────────┴──────────────────┐
        │                                     │
        ▼                                     ▼
┌───────────────┐                    ┌───────────────┐
│  DMPool       │                    │  DMPool Admin │
│  (Hydrapool)  │◄───────────────────│  (管理后台)   │
│  Port: 3333   │   共享数据库        │  Port: 8080   │
│  API: 8081    │                    │  (内网访问)    │
└───────────────┘                    └───────────────┘
        │                                     │
        └──────────────────┬──────────────────┘
                           ▼
            ┌────────────────────────┐
            │   PostgreSQL           │
            │   (共享数据库)          │
            └────────────────────────┘
```

---

## 前提条件

### 硬件要求
- CPU: 4核心以上
- 内存: 8GB 以上
- 存储: 100GB 以上 (包含 Bitcoin 节点数据)

### 软件要求
- Docker 20.10+
- Docker Compose 2.0+
- Bitcoin Core 25.0+ (全节点)
- Nginx 1.24+

---

## 部署步骤

### 1. 准备 Bitcoin 节点

确保 Bitcoin 节点已同步完成，并配置 RPC 访问：

```ini
# bitcoin.conf
server=1
rpcuser=dmpool
rpcpassword=your_secure_password
rpcbind=0.0.0.0
rpcallowip=172.16.0.0/12  # Docker 网络

# ZMQ (必需)
zmqpubhashblock=tcp://0.0.0.0:28332

# 为 Coinbase 交易预留空间
blockmaxweight=3930000  # 支持 ~500 个 P2PKH 输出
```

### 2. 配置 DMPool

编辑 `config.toml`:

```toml
# Bitcoin RPC 配置
[bitcoinrpc]
url = "http://host.docker.internal:8332"
username = "dmpool"
password = "your_secure_password"

# ZMQ 配置
[stratum]
zmqpubhashblock = "tcp://host.docker.internal:28332"
hostname = "0.0.0.0"
port = 3333
network = "signet"  # 或 "bitcoin" (主网)

# API 配置
[api]
hostname = "0.0.0.0"
port = 8081

# 存储配置
[store]
path = "/app/data"
background_task_frequency_hours = 1
pplns_ttl_days = 7

# 日志配置
[logging]
stats_dir = "/app/data/stats"
```

### 3. 配置环境变量

创建 `.env` 文件:

```bash
# 数据库密码
DB_PASSWORD=your_secure_db_password

# 管理员密码
ADMIN_PASSWORD=Admin@2026!

# 日志级别
RUST_LOG=info
RUST_BACKTRACE=0
```

### 4. 启动服务

```bash
# 构建并启动所有服务
docker compose up -d

# 查看日志
docker compose logs -f dmpool

# 检查服务状态
docker compose ps
```

### 5. 验证部署

```bash
# 检查健康状态
curl http://localhost:8081/health

# 检查 PPLNS 份额
curl http://localhost:8081/pplns_shares

# 检查管理后台 (仅内网)
curl http://localhost:8080/
```

---

## API 端点

### Observer API (公开访问)

| 端点 | 方法 | 描述 | 认证 |
|------|------|------|------|
| `/api/v1/stats/health` | GET | 健康检查 | 无 |
| `/api/v1/stats/metrics` | GET | Prometheus 格式指标 | 无 |
| `/api/v1/stats/shares` | GET | PPLNS 份额数据 | 无 |

**注意**: p2poolv2_api 使用 Basic Auth，需要在 Nginx 层移除或配置公开端点。

### Admin API (内网访问)

| 端点 | 方法 | 描述 | 认证 |
|------|------|------|------|
| `/` | GET | 管理后台首页 | JWT |
| `/api/admin/*` | * | 管理功能 | JWT |

**网络隔离**: 仅允许 192.168.0.0/16, 172.16.0.0/12, 10.0.0.0/8, 100.64.0.0/10 访问

---

## 监控和维护

### 查看日志

```bash
# DMPool 主服务
docker compose logs -f dmpool

# PostgreSQL
docker compose logs -f postgres

# Nginx
docker compose logs -f nginx
```

### 备份数据

```bash
# 停止服务
docker compose down

# 备份数据卷
docker run --rm -v dmpool_postgres_data:/data -v $(pwd):/backup \
  alpine tar czf /backup/postgres_backup_$(date +%Y%m%d).tar.gz -C /data .

# 重启服务
docker compose up -d
```

### 升级

```bash
# 拉取最新镜像
docker compose pull

# 重启服务
docker compose up -d --force-recreate
```

---

## 安全建议

1. **防火墙配置**
   - 只开放必要端口 (80, 443)
   - 管理后台端口 8080 只监听 127.0.0.1

2. **SSL 证书**
   - 使用 Let's Encrypt 获取免费证书
   - 或使用 Cloudflare Proxy

3. **定期更新**
   - 保持 Docker 镜像最新
   - 及时更新系统补丁

4. **监控告警**
   - 配置 Prometheus + Grafana
   - 设置磁盘空间、CPU、内存告警

---

## 故障排查

### DMPool 无法连接 Bitcoin 节点

```bash
# 检查 Bitcoin 节点状态
bitcoin-cli getblockchaininfo

# 检查 ZMQ 是否正常
netstat -an | grep 28332

# 测试 RPC 连接
curl --user dmpool:password --data-binary '{"jsonrpc": "1.0", "id":"curltest", "method": "getblockchaininfo", "params": [] }' -H 'content-type: text/plain;' http://localhost:8332/
```

### Stratum 连接失败

```bash
# 检查端口监听
netstat -an | grep 3333

# 查看矿池日志
docker compose logs dmpool | grep stratum
```

### API 返回 401

检查 `config.toml` 中的 `auth_user` 和 `auth_token` 配置。

---

## 参考资料

- [DMPool PRD v2.0](../DMPool_Decentralized_AI_Solution_v2.md)
- [Hydrapool 官方文档](https://github.com/256foundation/hydrapool)
- [P2Poolv2 API 文档](https://github.com/p2poolv2/p2poolv2)
