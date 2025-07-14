# WSL Prosody Setup Script for NTO
# Run this inside WSL to set up Prosody for development

Write-Host "🔧 Setting up Prosody XMPP Server for NTO Development"
Write-Host "═══════════════════════════════════════════════════════════"

$ErrorActionPreference = "Stop"

# Function to check if we're running in WSL
function Test-WSLEnvironment {
    return (Test-Path "/proc/version") -and (Get-Content "/proc/version" | Select-String -Pattern "WSL|Microsoft")
}

# Check if we're in WSL
if (-not (Test-WSLEnvironment)) {
    Write-Host "❌ This script must be run inside WSL" -ForegroundColor Red
    Write-Host "💡 Start WSL with: wsl" -ForegroundColor Yellow
    exit 1
}

Write-Host "✅ Running inside WSL environment" -ForegroundColor Green

# Install Prosody
Write-Host "`n📥 Installing Prosody XMPP Server..." -ForegroundColor Cyan
bash -c "
    # Update package list
    sudo apt update

    # Install required packages
    sudo apt install -y prosody lua-unbound

    # Check if installation was successful
    if command -v prosody >/dev/null 2>&1; then
        echo '✅ Prosody installed successfully'
        prosody --version
    else
        echo '❌ Prosody installation failed'
        exit 1
    fi
"

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Failed to install Prosody" -ForegroundColor Red
    exit 1
}

# Create configuration
Write-Host "`n⚙️ Creating Prosody configuration..." -ForegroundColor Cyan

$configContent = @"
-- Prosody Configuration for NTO Development
-- Optimized for local development with WSL

---------- Server-wide settings ----------
data_path = "/var/lib/prosody"

log = {
    info = "/var/log/prosody/prosody.log";
    error = "/var/log/prosody/prosody.err";
}

-- Network settings
c2s_ports = { 5222 }
s2s_ports = { 5269 }
http_ports = { 5280 }
https_ports = { 5281 }

-- Listen on all interfaces for WSL
c2s_interfaces = { "0.0.0.0", "::" }
s2s_interfaces = { "0.0.0.0", "::" }

---------- Modules ----------
modules_enabled = {
    -- Generally required
    "roster";
    "saslauth";
    "tls";
    "dialback";
    "disco";

    -- Not essential, but recommended
    "carbons";
    "pep";
    "private";
    "blocklist";
    "vcard4";
    "vcard_legacy";

    -- Nice to have
    "version";
    "uptime";
    "time";
    "ping";
    "register";
    "mam";

    -- Admin interfaces
    "admin_adhoc";
    "admin_shell";

    -- HTTP modules
    "bosh";
    "websocket";
    "http_files";

    -- Other specific functionality
    "groups";
    "server_contact_info";
    "announce";
    "welcome";
    "watchregistrations";
    "motd";
}

modules_disabled = {}

---------- Authentication ----------
authentication = "internal_hashed"
allow_registration = true

---------- Virtual hosts ----------
VirtualHost "nto.local"
    enabled = true
    contact_info = {
        abuse = { "admin", "nto.local" };
        admin = { "admin", "nto.local" };
    }

---------- Components ----------
Component "groups.nto.local" "muc"
    modules_enabled = {
        "muc_mam";
    }

---------- SSL/TLS Configuration ----------
ssl = {
    certificate = "/etc/prosody/certs/nto.local.crt";
    key = "/etc/prosody/certs/nto.local.key";
}

---------- Advanced settings ----------
c2s_timeout = 300
s2s_timeout = 300
log_level = "debug"
consider_bosh_secure = true
consider_websocket_secure = true

admins = { "admin@nto.local" }

---------- Archive configuration ----------
default_archive_policy = "roster"
max_archive_query_results = 50
archive_expires_after = "1w"
"@

# Write configuration file
bash -c "
    # Backup original config
    sudo cp /etc/prosody/prosody.cfg.lua /etc/prosody/prosody.cfg.lua.backup 2>/dev/null || true

    # Write new configuration
    echo '$configContent' | sudo tee /etc/prosody/prosody.cfg.lua > /dev/null

    echo '✅ Configuration file created'
"

# Create SSL certificates
Write-Host "`n🔐 Creating SSL certificates..." -ForegroundColor Cyan
bash -c "
    # Create certificates directory
    sudo mkdir -p /etc/prosody/certs

    # Generate self-signed certificate
    sudo openssl req -x509 -newkey rsa:4096 \
        -keyout /etc/prosody/certs/nto.local.key \
        -out /etc/prosody/certs/nto.local.crt \
        -days 365 -nodes \
        -subj '/CN=nto.local/O=NTO Development/C=US'

    # Set proper permissions
    sudo chown -R prosody:prosody /etc/prosody/certs
    sudo chmod 600 /etc/prosody/certs/*.key
    sudo chmod 644 /etc/prosody/certs/*.crt

    echo '✅ SSL certificates created'
"

# Test configuration
Write-Host "`n🧪 Testing configuration..." -ForegroundColor Cyan
bash -c "
    # Check configuration syntax
    sudo prosodyctl check config

    if [ \$? -eq 0 ]; then
        echo '✅ Configuration is valid'
    else
        echo '❌ Configuration has errors'
        exit 1
    fi
"

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Configuration validation failed" -ForegroundColor Red
    exit 1
}

# Start Prosody
Write-Host "`n🚀 Starting Prosody..." -ForegroundColor Cyan
bash -c "
    # Enable and start Prosody
    sudo systemctl enable prosody
    sudo systemctl start prosody

    # Wait a moment for startup
    sleep 3

    # Check status
    if sudo systemctl is-active prosody >/dev/null 2>&1; then
        echo '✅ Prosody is running'
        sudo systemctl status prosody --no-pager -l
    else
        echo '❌ Prosody failed to start'
        sudo journalctl -u prosody.service --no-pager -l
        exit 1
    fi
"

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Failed to start Prosody" -ForegroundColor Red
    exit 1
}

# Create admin user
Write-Host "`n👤 Creating admin user..." -ForegroundColor Cyan
bash -c "
    echo 'Creating admin user (admin@nto.local)...'
    echo 'Please enter a password for the admin user:'
    sudo prosodyctl adduser admin@nto.local
"

# Create test users
Write-Host "`n👥 Creating test users..." -ForegroundColor Cyan
bash -c "
    echo 'Creating test users with default password: test123'
    
    # Create test users with default password
    echo 'test123' | sudo prosodyctl adduser alice@nto.local --password-stdin 2>/dev/null || {
        echo 'alice@nto.local'
        echo 'test123'
        echo 'test123'
    } | sudo prosodyctl adduser alice@nto.local
    
    echo 'test123' | sudo prosodyctl adduser bob@nto.local --password-stdin 2>/dev/null || {
        echo 'bob@nto.local'
        echo 'test123'
        echo 'test123'
    } | sudo prosodyctl adduser bob@nto.local
    
    echo 'test123' | sudo prosodyctl adduser charlie@nto.local --password-stdin 2>/dev/null || {
        echo 'charlie@nto.local'
        echo 'test123'
        echo 'test123'
    } | sudo prosodyctl adduser charlie@nto.local
    
    echo '✅ Test users created:'
    echo '   - alice@nto.local (password: test123)'
    echo '   - bob@nto.local (password: test123)'
    echo '   - charlie@nto.local (password: test123)'
"

# Create helper script
Write-Host "`n🛠️ Creating helper script..." -ForegroundColor Cyan
bash -c "
    cat > ~/emi-prosody.sh << 'EOF'
#!/bin/bash
# NTO Prosody Helper Script

case \"\$1\" in
    start)
        echo 'Starting Prosody...'
        sudo systemctl start prosody
        sudo systemctl status prosody --no-pager
        ;;
    stop)
        echo 'Stopping Prosody...'
        sudo systemctl stop prosody
        ;;
    restart)
        echo 'Restarting Prosody...'
        sudo systemctl restart prosody
        sudo systemctl status prosody --no-pager
        ;;
    status)
        sudo systemctl status prosody --no-pager
        ;;
    logs)
        echo 'Showing Prosody logs (Ctrl+C to exit)...'
        sudo tail -f /var/log/prosody/prosody.log
        ;;
    config)
        echo 'Checking configuration...'
        sudo prosodyctl check config
        ;;
    users)
        echo 'Available commands for user management:'
        echo '  sudo prosodyctl adduser user@nto.local'
        echo '  sudo prosodyctl deluser user@nto.local'
        echo '  sudo prosodyctl passwd user@nto.local'
        ;;
    adduser)
        if [ -z \"\$2\" ]; then
            echo 'Usage: \$0 adduser username'
            echo 'Will create username@nto.local'
        else
            sudo prosodyctl adduser \"\$2@nto.local\"
        fi
        ;;
    *)
        echo 'NTO Prosody Helper'
        echo 'Usage: \$0 {start|stop|restart|status|logs|config|users|adduser}'
        echo ''
        echo 'Commands:'
        echo '  start    - Start Prosody service'
        echo '  stop     - Stop Prosody service'
        echo '  restart  - Restart Prosody service'
        echo '  status   - Show service status'
        echo '  logs     - Show live logs'
        echo '  config   - Check configuration'
        echo '  users    - Show user management help'
        echo '  adduser  - Add a new user'
        ;;
esac
EOF

    chmod +x ~/emi-prosody.sh
    echo '✅ Helper script created at ~/emi-prosody.sh'
"

# Final status check
Write-Host "`n📊 Final status check..." -ForegroundColor Cyan
bash -c "
    echo 'Service Status:'
    sudo systemctl is-active prosody
    
    echo ''
    echo 'Port Status:'
    sudo netstat -tlnp | grep :5222 || echo 'Port 5222 not found'
    
    echo ''
    echo 'Recent logs:'
    sudo tail -n 5 /var/log/prosody/prosody.log 2>/dev/null || echo 'No logs yet'
"

Write-Host "`n🎉 Prosody setup completed successfully!" -ForegroundColor Green
Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Green
Write-Host ""
Write-Host "📝 Summary:" -ForegroundColor Cyan
Write-Host "  • Prosody XMPP server is installed and running"
Write-Host "  • Server domain: nto.local"
Write-Host "  • XMPP port: 5222 (accessible from Windows)"
Write-Host "  • Admin user: admin@nto.local"
Write-Host "  • Test users: alice, bob, charlie @nto.local (password: test123)"
Write-Host "  • Helper script: ~/emi-prosody.sh"
Write-Host ""
Write-Host "🎯 Next steps:" -ForegroundColor Yellow
Write-Host "  1. Add to Windows hosts file: 127.0.0.1 nto.local"
Write-Host "  2. Run from Windows: .\scripts\start-xmpp-dev.ps1"
Write-Host "  3. Start developing your XMPP integration!"
Write-Host ""
Write-Host "💡 Helpful commands:" -ForegroundColor Gray
Write-Host "  ~/emi-prosody.sh status   - Check server status"
Write-Host "  ~/emi-prosody.sh logs     - View live logs"
Write-Host "  ~/emi-prosody.sh restart  - Restart server"
