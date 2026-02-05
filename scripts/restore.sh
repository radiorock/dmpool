#!/bin/bash
# DMPool Backup Restore Script
# Restores database and configuration files from a backup

set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <backup_date>"
    echo "Example: $0 20260203_182603"
    echo ""
    echo "Available backups:"
    ls -1 /home/k0n9/dmpool/backups/*.tar.gz 2>/dev/null | sed 's/.*\///' | sed 's/.tar.gz//' | tail -5
    exit 1
fi

BACKUP_DATE="$1"
BACKUP_ROOT="/home/k0n9/dmpool/backups"
DMP_DIR="/home/k0n9/dmpool"
BACKUP_FILE="$BACKUP_ROOT/$BACKUP_DATE.tar.gz"

# Check if backup exists
if [ ! -f "$BACKUP_FILE" ]; then
    echo "Error: Backup file not found: $BACKUP_FILE"
    exit 1
fi

# Confirm restore
echo "This will RESTORE the following backup:"
echo "  File: $BACKUP_FILE"
echo "  To: $DMP_DIR"
echo ""
echo "⚠️  WARNING: This will OVERWRITE existing data!"
echo ""
read -p "Are you sure? (type 'yes' to confirm): " confirm

if [ "$confirm" != "yes" ]; then
    echo "Restore cancelled."
    exit 0
fi

echo "[$(date)] Starting restore..."

# Stop services
echo "Stopping DMPool services..."
sudo systemctl stop dmpool.service dmpool-admin.service 2>/dev/null || true

# Create temporary restore directory
TEMP_DIR=$(mktemp -d)
echo "Extracting backup to $TEMP_DIR..."

# Extract backup
tar -xzf "$BACKUP_FILE" -C "$TEMP_DIR"

# Find the backup directory (it should be inside)
RESTORE_DIR=$(find "$TEMP_DIR" -type d -name "$BACKUP_DATE" | head -1)

if [ -z "$RESTORE_DIR" ]; then
    echo "Error: Could not find backup directory in archive"
    rm -rf "$TEMP_DIR"
    exit 1
fi

# Restore data directory
echo "Restoring data directory..."
if [ -d "$RESTORE_DIR/data" ]; then
    rm -rf "$DMP_DIR/data"
    cp -r "$RESTORE_DIR/data" "$DMP_DIR/"
fi

# Restore config files
echo "Restoring configuration files..."
if [ -f "$RESTORE_DIR/homelab.toml" ]; then
    cp "$RESTORE_DIR/homelab.toml" "$DMP_DIR/config/"
fi
if [ -f "$RESTORE_DIR/homelab-signet.toml" ]; then
    cp "$RESTORE_DIR/homelab-signet.toml" "$DMP_DIR/config/"
fi
if [ -f "$RESTORE_DIR/dmpool.conf" ]; then
    cp "$RESTORE_DIR/dmpool.conf" "$DMP_DIR/"
fi
if [ -f "$RESTORE_DIR/dmpool-admin.conf" ]; then
    cp "$RESTORE_DIR/dmpool-admin.conf" "$DMP_DIR/"
fi

# Clean up
rm -rf "$TEMP_DIR"

# Restart services
echo "Starting DMPool services..."
sudo systemctl start dmpool.service dmpool-admin.service

echo "[$(date)] Restore completed successfully!"
echo ""
echo "Please verify the system is working correctly:"
echo "  systemctl status dmpool.service"
echo "  systemctl status dmpool-admin.service"

exit 0
