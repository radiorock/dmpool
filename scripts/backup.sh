#!/bin/bash
# DMPool Automated Backup Script
# Backs up database and configuration files

set -e

# Configuration
BACKUP_ROOT="/home/k0n9/dmpool/backups"
DMP_DIR="/home/k0n9/dmpool"
RETENTION_DAYS=30
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="$BACKUP_ROOT/$DATE"

# Create backup directory
mkdir -p "$BACKUP_DIR"

echo "[$(date)] Starting backup..."

# Backup data directory
echo "Backing up data directory..."
mkdir -p "$BACKUP_DIR/data"
cp -r "$DMP_DIR/data/"* "$BACKUP_DIR/data/" 2>/dev/null || true

# Backup config files
echo "Backing up configuration..."
cp "$DMP_DIR/config/homelab.toml" "$BACKUP_DIR/"
cp "$DMP_DIR/config/homelab-signet.toml" "$BACKUP_DIR/" 2>/dev/null || true
cp "$DMP_DIR/dmpool.conf" "$BACKUP_DIR/" 2>/dev/null || true
cp "$DMP_DIR/dmpool-admin.conf" "$BACKUP_DIR/" 2>/dev/null || true

# Create tarball
echo "Creating compressed archive..."
tar -czf "$BACKUP_DIR.tar.gz" -C "$BACKUP_ROOT" "$DATE"

# Remove uncompressed backup
rm -rf "$BACKUP_DIR"

# Clean old backups
echo "Cleaning old backups (keeping $RETENTION_DAYS days)..."
find "$BACKUP_ROOT" -name "*.tar.gz" -mtime +$RETENTION_DAYS -delete

# Log completion
BACKUP_SIZE=$(du -h "$BACKUP_DIR.tar.gz" | cut -f1)
echo "[$(date)] Backup completed: $BACKUP_DIR.tar.gz ($BACKUP_SIZE)"

# Optional: Send to remote server
# rsync -avz "$BACKUP_DIR.tar.gz" backup-server:/backups/dmpool/

exit 0
