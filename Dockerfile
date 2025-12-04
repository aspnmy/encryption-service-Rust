# 使用官方的Rust镜像作为基础镜像
FROM docker.io/library/rust:1.91.1-slim AS builder

# 设置工作目录
WORKDIR /app

# 复制Cargo.toml和Cargo.lock
COPY Cargo.toml Cargo.lock ./

# 使用cargo fetch获取依赖，这样不需要完整的源码结构
RUN cargo fetch

# 复制实际的源代码
COPY src ./src

# 在构建阶段设置必要的环境变量，但不包含敏感信息
ENV CRUD_API_WRITE_INSTANCE_URL=http://localhost:8000
ENV CRUD_API_READ_INSTANCE_URL=http://localhost:8000
ENV JWT_SECRET=temp_build_secret

# 构建发布版本，cargo build本身只会编译代码，不会运行测试或可执行文件
RUN cargo build --release

# 使用轻量级的Alpine镜像作为最终镜像
FROM alpine:latest

# 安装必要的依赖
RUN apk --no-cache add ca-certificates

# 从builder阶段复制编译好的二进制文件
COPY --from=builder /app/target/release/encryption-service /usr/local/bin/encryption-service

# 设置环境变量
ENV RUST_LOG=info
ENV CRUD_API_BACKEND_TYPE=read_write_split
ENV CRUD_API_WRITE_INSTANCE_URL=http://10.168.3.165:7981
ENV CRUD_API_READ_INSTANCE_URL=http://10.168.3.165:7982
ENV JWT_SECRET=a1v0t7BjeTPKjQgeQMummRWEfJmc8sY1

# 暴露端口
EXPOSE 9999

# 设置入口点
ENTRYPOINT ["/usr/local/bin/encryption-service"]
