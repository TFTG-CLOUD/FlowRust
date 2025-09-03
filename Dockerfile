FROM ubuntu:22.04

# 设置环境变量
ENV DEBIAN_FRONTEND=noninteractive
ENV MONGODB_VERSION=8.0
ENV MONGODB_PACKAGE=mongodb-org

# 安装必要的工具
RUN apt-get update && apt-get install -y \
    wget \
    unzip \
    curl \
    gnupg \
    systemctl \
    supervisor \
    openssl \
    && rm -rf /var/lib/apt/lists/*

# 添加 MongoDB GPG 密钥
RUN curl -fsSL https://www.mongodb.org/static/pgp/server-${MONGODB_VERSION}.asc | gpg -o /usr/share/keyrings/mongodb-server-${MONGODB_VERSION}.gpg --dearmor

# 添加 MongoDB 仓库
RUN echo "deb [ arch=amd64,arm64 signed-by=/usr/share/keyrings/mongodb-server-${MONGODB_VERSION}.gpg ] https://repo.mongodb.org/apt/ubuntu jammy/mongodb-org/${MONGODB_VERSION} multiverse" | tee /etc/apt/sources.list.d/mongodb-org-${MONGODB_VERSION}.list

# 安装 MongoDB
RUN apt-get update && apt-get install -y ${MONGODB_PACKAGE} \
    && rm -rf /var/lib/apt/lists/*

# 复制启动脚本
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

# 设置二进制文件执行权限
RUN chmod +x /usr/bin/mongod /usr/local/bin/entrypoint.sh

# 暴露端口
EXPOSE 8080

# 启动 Supervisor
CMD ["/usr/local/bin/entrypoint.sh"]