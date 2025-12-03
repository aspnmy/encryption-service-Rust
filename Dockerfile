FROM rust:1.79.0-slim-buster AS builder

# 设置工作目录
WORKDIR /app

# 复制依赖文件
COPY Cargo.toml Cargo.lock ./

# 复制源代码
COPY src ./src

# 构建应用
RUN cargo build --release

# 创建运行时镜像
FROM debian:buster-slim

# 安装依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# 设置工作目录
WORKDIR /app

# 从构建阶段复制可执行文件
COPY --from=builder /app/target/release/encryption-service ./

# 复制环境变量示例文件
COPY .env.example ./

# 暴露端口
EXPOSE 8080

# 运行应用
CMD ["./encryption-service"]
