#!/bin/bash

# 创建必要的日志目录
mkdir -p /var/log/mongodb /var/log/flowrust_cms

# 创建 MongoDB 数据目录
mkdir -p /var/lib/mongodb

if [ ! -f "/app/flowrust_cms" ]; then
    # 创建必要的目录
    mkdir -p /app/static/images

    # 下载最新的 flowrust_cms 版本
    cd /app && wget -O linux.zip https://github.com/TFTG-CLOUD/FlowRust/releases/latest/download/linux.zip \
        && unzip linux.zip \
        && rm linux.zip
fi

chmod +x /app/flowrust_cms

# 生成随机key
SESSION_KEY=$(openssl rand -hex 32)

# 设置默认值，如果环境变量未设置则使用默认值
ADMIN_USER="${ADMIN_USER:-admin}"
ADMIN_PASS="${ADMIN_PASS:-password123}"

# 创建 .env 文件
cat > /app/.env << EOF
DATABASE_URL=mongodb://localhost:27017
DATABASE_NAME=flowrust_cms
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
ADMIN_USER=${ADMIN_USER}
ADMIN_PASS=${ADMIN_PASS}
SESSION_SECRET_KEY=${SESSION_KEY}
RUST_LOG=info
EOF

# 复制 Supervisor 配置
cat > /etc/supervisor/conf.d/supervisord.conf <<EOF
[supervisord]
nodaemon=true

[program:mongodb]
command=bash -c "rm -rf /var/lib/mongodb/mongod.lock /tmp/mongodb-27017.sock /var/lib/mongodb/WiredTiger.lock && /usr/bin/mongod --dbpath /var/lib/mongodb --logpath /var/log/mongodb/mongodb.log"
priority=0
autostart=true
autorestart=true
stderr_logfile=/var/log/mongodb/mongodb_stderr.log
stdout_logfile=/var/log/mongodb/mongodb_stdout.log

[program:flowrust_cms]
command=/app/flowrust_cms
directory=/app
priority=1
autostart=true
autorestart=true
stdout_logfile=/var/log/flowrust_cms/flowrust_cms.log
stderr_logfile=/var/log/flowrust_cms/flowrust_cms.err
EOF

exec supervisord -c /etc/supervisor/conf.d/supervisord.conf
