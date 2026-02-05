#!/bin/bash
# DMPool Deployment Script for Homelab
# Target: 192.168.5.21 (homelab server)

set -e

# Configuration
REMOTE_USER="k0n9"
REMOTE_HOST="192.168.5.21"
PROJECT_DIR="/home/k0n9/dmpool"
BUILD_DIR="dist"
SERVICE_NAME="dmpool"

echo "🚀 Starting DMPool deployment to homelab..."

# Build Observer Frontend (React)
echo "📦 Building React Observer Frontend..."
cd /Users/K4y/work/dmpool/dmpool-rust/web-observer
npm run build

# Build Admin Frontend (Vue)
echo "📦 Building Vue Admin Frontend..."
cd /Users/K4y/work/dmpool/dmpool-rust/web-admin
npm run build

# Build backend
echo "📦 Building Rust Backend..."
cd /Users/K4y/work/dmpool/dmpool-rust
cargo build --release

# Create directories on remote
echo "📁 Setting up remote directories..."
ssh ${REMOTE_USER}@${REMOTE_HOST} "mkdir -p ${PROJECT_DIR}/backend ${PROJECT_DIR}/config"

# Deploy backend binary
echo "📤 Deploying backend binary..."
scp target/release/dmpool ${REMOTE_USER}@${REMOTE_HOST}:${PROJECT_DIR}/backend/

# Deploy Observer Frontend (React)
echo "📤 Deploying React Observer Frontend..."
ssh ${REMOTE_USER}@${REMOTE_HOST} "sudo rm -rf /var/www/dmpool && sudo mkdir -p /var/www/dmpool"
scp -r web-observer/${BUILD_DIR}/* ${REMOTE_USER}@${REMOTE_HOST}:/tmp/dmpool-frontend/
ssh ${REMOTE_USER}@${REMOTE_HOST} "sudo mv /tmp/dmpool-frontend/* /var/www/dmpool/ && sudo chown -R www-data:www-data /var/www/dmpool"

# Deploy Admin Frontend (Vue)
echo "📤 Deploying Vue Admin Frontend..."
ssh ${REMOTE_USER}@${REMOTE_HOST} "sudo rm -rf /var/www/dmpool-admin && sudo mkdir -p /var/www/dmpool-admin"
scp -r web-admin/${BUILD_DIR}/* ${REMOTE_USER}@${REMOTE_HOST}:/tmp/dmpool-admin/
ssh ${REMOTE_USER}@${REMOTE_HOST} "sudo mv /tmp/dmpool-admin/* /var/www/dmpool-admin/ && sudo chown -R www-data:www-data /var/www/dmpool-admin"

# Deploy Nginx config
echo "📤 Deploying Nginx configuration..."
scp deploy/nginx-dmpool.conf ${REMOTE_USER}@${REMOTE_HOST}:/tmp/
ssh ${REMOTE_USER}@${REMOTE_HOST} "sudo mv /tmp/nginx-dmpool.conf /etc/nginx/sites-available/dmpool && sudo ln -sf /etc/nginx/sites-available/dmpool /etc/nginx/sites-enabled/ && sudo nginx -t && sudo systemctl reload nginx"

# Deploy systemd service
echo "📤 Deploying systemd service..."
cat > /tmp/dmpool.service << 'EOF'
[Unit]
Description=DMPool - Decentralized Bitcoin Mining Pool
After=network.target postgresql.service

[Service]
Type=simple
User=k0n9
WorkingDirectory=/home/k0n9/dmpool/backend
Environment="RUST_LOG=info"
Environment="DATABASE_URL=postgresql://dmpool:dmpool@localhost:5432/dmpool"
Environment="OBSERVER_API_HOST=0.0.0.0"
Environment="OBSERVER_API_PORT=8082"
Environment="ADMIN_API_HOST=127.0.0.1"
Environment="ADMIN_API_PORT=8080"
ExecStart=/home/k0n9/dmpool/backend/dmpool --config /home/k0n9/dmpool/config/dmpool.toml
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF
scp /tmp/dmpool.service ${REMOTE_USER}@${REMOTE_HOST}:/tmp/
ssh ${REMOTE_USER}@${REMOTE_HOST} "sudo mv /tmp/dmpool.service /etc/systemd/system/ && sudo systemctl daemon-reload && sudo systemctl enable dmpool"

echo "✅ Deployment complete!"
echo ""
echo "Next steps:"
echo "1. Copy your dmpool.toml config to ${PROJECT_DIR}/config/"
echo "2. Set up PostgreSQL database:"
echo "   - sudo -u postgres createuser dmpool"
echo "   - sudo -u postgres psql -c \"ALTER USER dmpool PASSWORD 'dmpool';\""
echo "   - sudo -u postgres createdb -O dmpool dmpool"
echo "3. Start the service: ssh ${REMOTE_USER}@${REMOTE_HOST} 'sudo systemctl start dmpool'"
echo "4. Check status: ssh ${REMOTE_USER}@${REMOTE_HOST} 'sudo systemctl status dmpool'"
echo ""
echo "🌐 Observer Frontend: http://${REMOTE_HOST}/"
echo "🔧 Admin Panel: http://${REMOTE_HOST}/admin/ (VPN/internal only)"
echo "📊 Admin API: http://${REMOTE_HOST}/admin/api/ (VPN/internal only)"
