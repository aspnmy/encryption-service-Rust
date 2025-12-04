# 使用官方的Rust镜像作为基础镜像
FROM rust:1.81.0 AS builder

# 设置工作目录
WORKDIR /app

# 复制Cargo.toml和Cargo.lock
COPY Cargo.toml Cargo.lock ./

# 创建一个虚拟的src目录和main.rs文件，用于缓存依赖
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs

# 构建依赖
RUN cargo build --release

# 复制实际的源代码
COPY src ./src

# 再次构建，这次会使用实际的源代码
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
ENV CRUD_API_READ_INSTANCE_URL=http://10.168.3.168:7982
ENV JWT_SECRET=12345678901234567890

# 暴露端口
EXPOSE 8080

# 设置入口点
ENTRYPOINT ["/usr/local/bin/encryption-service"]
