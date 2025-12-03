# Encryption Service

一个基于 Rust 和 Axum 的加密服务，提供数据加密、解密和批量处理功能，支持多实例部署和服务角色分离。

## 架构设计

### 核心功能

- **数据加密**：支持使用 AES-256-GCM 算法加密数据
- **数据解密**：支持解密已加密的数据
- **批量加密**：支持批量加密多条数据
- **批量解密**：支持批量解密多条数据
- **服务角色支持**：支持 encrypt/decrypt/mixed 三种角色
- **CRUD API 集成**：自动与 CRUD API 服务交互，保存加密数据
- **健康检查**：提供服务健康状态检查
- **HTTPS 支持**：可配置启用 HTTPS

### 技术栈

- **语言**：Rust 2024
- **Web 框架**：Axum 0.7.9
- **异步运行时**：Tokio
- **序列化**：Serde
- **HTTP 客户端**：Reqwest
- **加密库**：AES-GCM、HKDF、SHA256
- **日志**：Tracing
- **配置管理**：Dotenvy

## 部署方式

### Docker 部署

```bash
# 构建镜像
docker build -t encryption-service .

# 运行容器 - 加密角色
docker run -d \
  --name encryption-service-encrypt \
  -p 8080:8080 \
  -e SERVICE_ROLE=encrypt \
  -e SERVICE_ID=encryption-encrypt-01 \
  -e CRUD_API_URL=http://crudapi-write:8000 \
  encryption-service

# 运行容器 - 解密角色
docker run -d \
  --name encryption-service-decrypt \
  -p 8081:8080 \
  -e SERVICE_ROLE=decrypt \
  -e SERVICE_ID=encryption-decrypt-01 \
  -e CRUD_API_URL=http://crudapi-read:8001 \
  encryption-service

# 运行容器 - 混合角色
docker run -d \
  --name encryption-service-mixed \
  -p 8082:8080 \
  -e SERVICE_ROLE=mixed \
  -e SERVICE_ID=encryption-mixed-01 \
  -e CRUD_API_URL=http://crudapi-write:8000 \
  encryption-service
```

### Docker Compose 部署

```yaml
# 参考项目根目录的 docker-compose.yml
```

## 配置说明

### 环境变量

| 变量名 | 描述 | 默认值 |
|--------|------|--------|
| `SERVER_HOST` | 服务器地址 | `0.0.0.0` |
| `SERVER_PORT` | 服务器端口 | `8080` |
| `HTTPS` | 是否启用 HTTPS | `false` |
| `JWT_SECRET` | JWT 密钥 | `your_secret_key` |
| `JWT_EXPIRES_IN` | JWT 过期时间（秒） | `3600` |
| `JWT_REFRESH_IN` | JWT 刷新时间（秒） | `86400` |
| `ENCRYPTION_ALGORITHM` | 加密算法 | `aes-256-gcm` |
| `ENCRYPTION_KEY_LENGTH` | 密钥长度 | `32` |
| `ENCRYPTION_ITERATIONS` | 迭代次数 | `100000` |
| `ENCRYPTION_SALT` | 加密盐值 | `default_salt` |
| `SERVICE_ROLE` | 服务角色（encrypt/decrypt/mixed） | `mixed` |
| `SERVICE_ID` | 服务 ID | `encryption-01` |
| `CRUD_API_URL` | CRUD API 服务 URL | `http://localhost:8000` |
| `CRUD_API_TIMEOUT` | CRUD API 超时时间（毫秒） | `5000` |
| `CRUD_API_RETRIES` | CRUD API 重试次数 | `3` |

## API 端点

### 健康检查

```
GET /health
```

### 加密端点

#### 加密数据

```
POST /encrypt

请求体：
{
  "data": "明文数据",
  "password": "加密密码",
  "resource_type": "资源类型"
}

响应体：
{
  "success": true,
  "message": "加密成功",
  "data": {
    "encrypted_data": "加密后的数据",
    "resource_id": "资源ID"
  }
}
```

#### 解密数据

```
POST /decrypt

请求体（使用资源ID）：
{
  "password": "解密密码",
  "resource_type": "资源类型",
  "resource_id": "资源ID"
}

或

请求体（直接提供加密数据）：
{
  "encrypted_data": "加密后的数据",
  "password": "解密密码",
  "resource_type": "资源类型"
}

响应体：
{
  "success": true,
  "message": "解密成功",
  "data": {
    "data": "解密后的明文数据",
    "resource_id": "资源ID"
  }
}
```

#### 批量加密

```
POST /batch/encrypt

请求体：
[
  {
    "data": "明文数据1",
    "password": "加密密码",
    "resource_type": "资源类型"
  },
  {
    "data": "明文数据2",
    "password": "加密密码",
    "resource_type": "资源类型"
  }
]

响应体：
{
  "success": true,
  "message": "批量加密成功",
  "data": [
    {
      "encrypted_data": "加密后的数据1",
      "resource_id": "资源ID1"
    },
    {
      "encrypted_data": "加密后的数据2",
      "resource_id": "资源ID2"
    }
  ]
}
```

#### 批量解密

```
POST /batch/decrypt

请求体：
[
  {
    "resource_id": "资源ID1",
    "password": "解密密码",
    "resource_type": "资源类型"
  },
  {
    "encrypted_data": "加密后的数据2",
    "password": "解密密码",
    "resource_type": "资源类型"
  }
]

响应体：
{
  "success": true,
  "message": "批量解密成功",
  "data": [
    {
      "data": "解密后的明文数据1",
      "resource_id": "资源ID1"
    },
    {
      "data": "解密后的明文数据2",
      "resource_id": null
    }
  ]
}
```

## 服务角色

### Encrypt 角色

- 仅允许执行加密操作
- 适用于加密密集型应用
- 可水平扩展，提高加密性能
- 与 CRUD API 写节点交互

### Decrypt 角色

- 仅允许执行解密操作
- 适用于解密密集型应用
- 可水平扩展，提高解密性能
- 与 CRUD API 读节点交互

### Mixed 角色

- 允许执行加密和解密操作
- 适用于开发环境或小型部署
- 可与 CRUD API 写节点或读节点交互

## 开发指南

### 本地开发

```bash
# 启动服务
cargo run
```

### 构建

```bash
# 构建开发版本
cargo build

# 构建发布版本
cargo build --release
```

### 测试

```bash
# 运行单元测试
cargo test

# 运行集成测试
cargo test -- --ignored
```

### 代码检查

```bash
# 检查语法错误
cargo check

# 格式化代码
cargo fmt

# 运行 clippy
cargo clippy
```

## 服务间通信

### 与 CRUD API 服务交互

- **加密数据保存**：加密服务将加密后的数据通过 HTTP POST 请求发送到 CRUD API 服务
- **加密数据获取**：解密服务通过 HTTP GET 请求从 CRUD API 服务获取加密数据
- **超时处理**：支持配置请求超时时间
- **重试机制**：支持配置请求重试次数

## 加密算法

### AES-256-GCM

- **算法类型**：对称加密算法
- **密钥长度**：256 位
- **模式**：Galois/Counter Mode (GCM)
- **特性**：提供认证加密，同时保证数据的机密性和完整性
- **nonce 长度**：12 字节，随机生成

### HKDF

- **用途**：从密码和盐生成加密密钥
- **哈希算法**：SHA256
- **输出长度**：32 字节（256 位）

## 安全最佳实践

1. **使用强密码**：加密密码应至少包含 16 个字符，包含大小写字母、数字和特殊字符
2. **定期更换密钥**：定期更换加密密钥和盐值
3. **使用 HTTPS**：在生产环境中启用 HTTPS
4. **限制服务访问**：通过网络策略限制服务间通信
5. **监控服务状态**：定期检查服务健康状态
6. **日志记录**：记录关键操作日志，便于审计和故障排查

## 贡献指南

1. Fork 项目
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 打开 Pull Request

## 许可证

MIT

## 联系信息

如有问题或建议，请创建 Issue 或提交 Pull Request。
