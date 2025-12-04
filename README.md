# Encryption Service 中间层

一个基于 Rust 和 Axum 的加密服务中间层，主要用于实现对后端通用 CRUD API 容器进行调度，支持单容器、读写分离和负载均衡三种模式，并提供完善的容错机制。

## 核心功能

### 调度功能

- **单容器模式**：读写操作均指向同一个 CRUD API 实例
- **读写分离模式**：写操作指向写实例，读操作指向读实例
- **负载均衡模式**：支持多个混合实例，自动分配请求

### 容错机制

- **数据缓存**：正常连接后端健康实例后，缓存当前数据到临时文件
- **应急实例**：当后端没有健康实例时，自动创建测试实例并导入缓存数据
- **定期更新**：每小时更新一次临时文件，删除 24 小时以前的临时数据
- **微信提醒**：测试实例存在超过 48 小时后，自动发送提醒到指定企业微信群

### 配置方式

- **环境变量驱动**：所有配置通过环境变量进行，无需修改代码
- **灵活的后端配置**：支持动态配置多个后端实例
- **无默认 URL**：容器启动时必须配置后端入口，否则抛出异常

## 技术栈

- **语言**：Rust 2024
- **Web 框架**：Axum 0.7.9
- **异步运行时**：Tokio
- **序列化**：Serde
- **HTTP 客户端**：Reqwest
- **加密库**：AES-GCM、HKDF、SHA256
- **日志**：Tracing
- **容器化**：Docker
- **CI/CD**：GitHub Actions

## 架构设计

### 调度模式

#### 1. 单容器模式

读写操作均指向同一个 CRUD API 实例，适用于小型部署或开发环境。

#### 2. 读写分离模式

- 写操作：仅使用配置的写实例
- 读操作：使用配置的读实例
- 适用于读多写少的场景，提高系统吞吐量

#### 3. 负载均衡模式

- 支持配置多个混合实例
- 写操作：轮询分配到所有混合实例
- 读操作：轮询分配到所有混合实例
- 适用于高并发场景，提高系统可用性和性能

### 容错机制流程

1. **正常运行**：加密服务连接到健康的 CRUD API 实例
2. **数据缓存**：定期将数据缓存到临时文件
3. **后端故障**：检测到所有 CRUD API 实例不可用
4. **创建测试实例**：自动创建测试实例，导入缓存数据
5. **数据写入**：后续请求写入到测试实例
6. **微信提醒**：测试实例存在超过 48 小时后发送提醒

## 部署方式

### Docker 部署

#### 构建镜像

```bash
docker build -t encryption-service .
```

#### 单容器模式

```bash
docker run -d \
  --name encryption-service-single \
  -p 9999:9999 \
  -e CRUD_API_BACKEND_TYPE=single \
  -e CRUD_API_WRITE_INSTANCE_URL=http://crudapi:8000 \
  -e JWT_SECRET=your_jwt_secret \
  encryption-service
```

#### 读写分离模式

```bash
docker run -d \
  --name encryption-service-rw \
  -p 9999:9999 \
  -e CRUD_API_BACKEND_TYPE=read_write_split \
  -e CRUD_API_WRITE_INSTANCE_URL=http://10.168.3.165:7981 \
  -e CRUD_API_READ_INSTANCE_URL=http://10.168.3.168:7982 \
  -e JWT_SECRET=your_jwt_secret \
  encryption-service
```

#### 负载均衡模式

```bash
docker run -d \
  --name encryption-service-lb \
  -p 9999:9999 \
  -e CRUD_API_BACKEND_TYPE=load_balance \
  -e CRUD_API_INSTANCE_0_ID=instance-01 \
  -e CRUD_API_INSTANCE_0_URL=http://crudapi-01:8000 \
  -e CRUD_API_INSTANCE_0_TYPE=mixed \
  -e CRUD_API_INSTANCE_1_ID=instance-02 \
  -e CRUD_API_INSTANCE_1_URL=http://crudapi-02:8000 \
  -e CRUD_API_INSTANCE_1_TYPE=mixed \
  -e JWT_SECRET=your_jwt_secret \
  encryption-service
```

## 环境变量配置

### 核心配置

| 变量名 | 描述 | 必填 | 默认值 |
|--------|------|------|--------|
| `CRUD_API_BACKEND_TYPE` | 后端类型：single/read_write_split/load_balance | 否 | read_write_split |
| `CRUD_API_WRITE_INSTANCE_URL` | 写实例 URL | 是 | - |
| `CRUD_API_READ_INSTANCE_URL` | 读实例 URL | 否 | 与写实例相同 |
| `JWT_SECRET` | JWT 密钥 | 是 | - |

### 单容器模式配置

| 变量名 | 描述 | 默认值 |
|--------|------|--------|
| `CRUD_API_WRITE_INSTANCE_TIMEOUT` | 写实例超时时间（毫秒） | 5000 |
| `CRUD_API_WRITE_INSTANCE_RETRIES` | 写实例重试次数 | 3 |

### 读写分离模式配置

| 变量名 | 描述 | 默认值 |
|--------|------|--------|
| `CRUD_API_WRITE_INSTANCE_TIMEOUT` | 写实例超时时间（毫秒） | 5000 |
| `CRUD_API_WRITE_INSTANCE_RETRIES` | 写实例重试次数 | 3 |
| `CRUD_API_READ_INSTANCE_TIMEOUT` | 读实例超时时间（毫秒） | 5000 |
| `CRUD_API_READ_INSTANCE_RETRIES` | 读实例重试次数 | 3 |

### 负载均衡模式配置

| 变量名 | 描述 | 默认值 |
|--------|------|--------|
| `CRUD_API_INSTANCE_{N}_ID` | 第 N 个实例 ID | - |
| `CRUD_API_INSTANCE_{N}_URL` | 第 N 个实例 URL | - |
| `CRUD_API_INSTANCE_{N}_TYPE` | 第 N 个实例类型：read/write/mixed | mixed |
| `CRUD_API_INSTANCE_{N}_TIMEOUT` | 第 N 个实例超时时间（毫秒） | 5000 |
| `CRUD_API_INSTANCE_{N}_RETRIES` | 第 N 个实例重试次数 | 3 |

### 其他配置

| 变量名 | 描述 | 默认值 |
|--------|------|--------|
| `SERVER_HOST` | 服务器地址 | 0.0.0.0 |
| `SERVER_PORT` | 服务器端口 | 9999 |
| `HTTPS` | 是否启用 HTTPS | false |
| `JWT_EXPIRES_IN` | JWT 过期时间（秒） | 3600 |
| `JWT_REFRESH_IN` | JWT 刷新时间（秒） | 86400 |
| `ENCRYPTION_ALGORITHM` | 加密算法 | aes-256-gcm |
| `ENCRYPTION_KEY_LENGTH` | 密钥长度 | 32 |
| `ENCRYPTION_ITERATIONS` | 迭代次数 | 100000 |
| `ENCRYPTION_SALT` | 加密盐值 | default_salt |
| `SERVICE_ROLE` | 服务角色：encrypt/decrypt/mixed | mixed |
| `SERVICE_ID` | 服务 ID | encryption-01 |
| `WECHAT_WEBHOOK_URL` | 企业微信群机器人 URL | - |

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

## 开发指南

### 本地开发

```bash
# 设置环境变量
export CRUD_API_WRITE_INSTANCE_URL=http://10.168.3.165:7981
export CRUD_API_READ_INSTANCE_URL=http://10.168.3.168:7982
export JWT_SECRET=your_jwt_secret

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

## CI/CD

项目包含 GitHub Actions 工作流，用于自动化构建和测试 Docker 镜像。

### 触发条件

- 推送至 `main`、`master`、`dev_rust` 分支
- 拉取请求至 `main`、`master`、`dev_rust` 分支
- 忽略 `*.md` 文件的更新

### 测试场景

- 单容器模式
- 读写分离模式
- 负载均衡模式

## 服务角色

### Encrypt 角色

- 仅允许执行加密操作
- 与 CRUD API 写节点交互
- 适用于加密密集型应用

### Decrypt 角色

- 仅允许执行解密操作
- 与 CRUD API 读节点交互
- 适用于解密密集型应用

### Mixed 角色

- 允许执行加密和解密操作
- 与 CRUD API 写节点或读节点交互
- 适用于开发环境或小型部署

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
7. **配置管理**：使用安全的方式管理环境变量，避免敏感信息泄露

## 容器化最佳实践

1. **最小化镜像**：使用 Alpine 基础镜像，减小镜像体积
2. **多阶段构建**：使用多阶段构建，减小最终镜像体积
3. **非 root 用户**：使用非 root 用户运行容器，提高安全性
4. **健康检查**：配置容器健康检查，确保容器正常运行
5. **资源限制**：配置容器资源限制，避免资源耗尽
6. **日志管理**：使用集中式日志管理，便于日志分析和监控

## 故障排查

### 容器启动失败

- 检查 `CRUD_API_WRITE_INSTANCE_URL` 环境变量是否设置
- 检查环境变量格式是否正确
- 检查后端实例是否可达

### 加密/解密失败

- 检查密码是否正确
- 检查资源类型是否匹配
- 检查 JWT 密钥是否正确
- 检查后端实例是否健康

### 健康检查失败

- 检查服务是否正常运行
- 检查后端实例是否可达
- 检查日志，查看具体错误信息

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