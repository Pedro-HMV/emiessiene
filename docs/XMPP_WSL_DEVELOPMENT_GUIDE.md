# XMPP Development with WSL - NTO Guide

## Overview

Since Prosody no longer provides Windows binaries, we'll use **Windows Subsystem for Linux (WSL)** to run the XMPP server during development. This guide explains the complete development workflow for integrating XMPP messaging with your NTO application.

## Development Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Windows Host  │    │      WSL 2      │    │   Development   │
│                 │    │                 │    │     Flow        │
├─────────────────┤    ├─────────────────┤    ├─────────────────┤
│ NTO App        │◄──►│ Prosody Server  │    │ 1. Start WSL    │
│ (Tauri/Leptos)  │    │ (Ubuntu/Debian) │    │ 2. Start Prosody│
│                 │    │                 │    │ 3. Start App    │
│ - Rust XMPP     │    │ - prosody.cfg   │    │ 4. Develop      │
│ - Connect to    │    │ - Local network │    │ 5. Test         │
│   localhost:5222│    │ - Port 5222     │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## WSL Setup for XMPP Development

### 1. Install WSL 2

```powershell
# Enable WSL and install Ubuntu
wsl --install -d Ubuntu

# Or if WSL is already enabled:
wsl --list --online
wsl --install -d Ubuntu-22.04
```

### 2. Configure WSL for Development

```bash
# Update system (run inside WSL)
sudo apt update && sudo apt upgrade -y

# Install essential tools
sudo apt install -y curl wget unzip build-essential
```

### 3. Install Prosody in WSL

```bash
# Add Prosody repository
wget https://prosody.im/files/prosody-debian-packages.key
sudo apt-key add prosody-debian-packages.key
echo 'deb http://packages.prosody.im/debian $(lsb_release -sc) main' | sudo tee /etc/apt/sources.list.d/prosody.list

# Install Prosody
sudo apt update
sudo apt install -y prosody
```

## Prosody Configuration

### Configuration File Location
**In WSL**: `/etc/prosody/prosody.cfg.lua`

### NTO Development Configuration

Create the configuration file:

```bash
# Edit the main config file
sudo nano /etc/prosody/prosody.cfg.lua
```

**Complete `prosody.cfg.lua` for NTO development**:

```lua
-- Prosody Configuration for NTO Development
-- This is optimized for local development on WSL

---------- Server-wide settings ----------

-- Information on configuring Prosody can be found on our
-- website at https://prosody.im/doc/configure

-- Prosody data directory
data_path = "/var/lib/prosody"

-- Logging configuration
log = {
    info = "/var/log/prosody/prosody.log";
    error = "/var/log/prosody/prosody.err";
}

-- Network settings
-- These ports will be accessible from Windows host
c2s_ports = { 5222 }       -- Client-to-Server (your app connects here)
s2s_ports = { 5269 }       -- Server-to-Server
http_ports = { 5280 }      -- HTTP (for admin interface)
https_ports = { 5281 }     -- HTTPS

-- Enable IPv4 and IPv6
c2s_interfaces = { "0.0.0.0", "::" }
s2s_interfaces = { "0.0.0.0", "::" }

---------- Modules ----------

-- This is the list of modules Prosody will load on startup.
modules_enabled = {
    -- Generally required
    "roster";                -- Allow users to have a roster (contact list)
    "saslauth";              -- Authentication for clients and servers
    "tls";                   -- Add support for secure TLS on c2s/s2s connections
    "dialback";              -- s2s dialback support
    "disco";                 -- Service discovery

    -- Not essential, but recommended
    "carbons";               -- Keep multiple clients in sync
    "pep";                   -- Enables users to publish their avatar, mood, activity, playing music and more
    "private";               -- Private XML storage (for bookmarks, etc.)
    "blocklist";             -- Allow users to block communications with other users
    "vcard4";                -- User profiles (stored in PEP)
    "vcard_legacy";          -- Conversion between legacy vCard and PEP Avatar, vcard

    -- Nice to have
    "version";               -- Replies to server version requests
    "uptime";                -- Report how long server has been running
    "time";                  -- Let others know the time here on this server
    "ping";                  -- Replies to XMPP pings with pongs
    "register";              -- Allow users to register on this server using a client and change passwords
    "mam";                   -- Store messages in an archive and allow users to access it
    "csi_simple";            -- Simple Mobile optimizations

    -- Admin interfaces
    "admin_adhoc";           -- Allows administration via an XMPP client that supports ad-hoc commands
    "admin_shell";           -- Allow secure administration via 'prosodyctl shell'

    -- HTTP modules
    "bosh";                  -- Enable BOSH clients, aka "Jabber over HTTP"
    "websocket";             -- XMPP over WebSockets
    "http_files";            -- Serve static files from a directory over HTTP

    -- Other specific functionality
    "groups";                -- Shared roster support
    "server_contact_info";   -- Publish contact information for this service
    "announce";              -- Send announcement to all online users
    "welcome";               -- Welcome users who register accounts
    "watchregistrations";    -- Alert admins of registrations
    "motd";                  -- Send a message of the day to users
}

-- These modules are auto-loaded, but should you want
-- to disable them then uncomment them here:
modules_disabled = {
    -- "offline"; -- Store offline messages
    -- "c2s";      -- Handle client connections
    -- "s2s";      -- Handle server-to-server connections
}

---------- Authentication ----------

-- Select the authentication backend to use.
authentication = "internal_hashed"

-- Allow account registration
allow_registration = true

---------- Virtual hosts ----------

-- NTO development domain
VirtualHost "nto.local"
    enabled = true
    
    -- Server contact information
    contact_info = {
        abuse = { "admin", "nto.local" };
        admin = { "admin", "nto.local" };
    }

---------- Components ----------

-- Set up a Multi-User Chat (MUC) (chatroom) service
Component "groups.nto.local" "muc"
    modules_enabled = {
        "muc_mam";           -- Store MUC messages in an archive and allow users to access it
    }

---------- SSL/TLS Configuration ----------

-- For development, we'll use self-signed certificates
-- Prosody will auto-generate these if they don't exist

ssl = {
    certificate = "/etc/prosody/certs/nto.local.crt";
    key = "/etc/prosody/certs/nto.local.key";
}

---------- Logging configuration ----------

-- Enable debug logging for development
log = {
    debug = "/var/log/prosody/prosody.log";
    error = "/var/log/prosody/prosody.err";
}

---------- Advanced settings ----------

-- For development, increase limits
c2s_timeout = 300
s2s_timeout = 300

-- Enable more verbose error reporting
log_level = "debug"

-- Development-friendly settings
consider_bosh_secure = true
consider_websocket_secure = true

-- Admin users (change this!)
admins = { "admin@nto.local" }

---------- Archive configuration ----------

-- Message archive settings
default_archive_policy = "roster"  -- Archive messages for contacts
max_archive_query_results = 50
archive_expires_after = "1w"       -- Keep messages for 1 week in development
```

### Set Up SSL Certificates (Self-Signed for Development)

```bash
# Create certificates directory
sudo mkdir -p /etc/prosody/certs

# Generate self-signed certificate for development
sudo openssl req -x509 -newkey rsa:4096 -keyout /etc/prosody/certs/nto.local.key -out /etc/prosody/certs/nto.local.crt -days 365 -nodes -subj "/CN=nto.local"

# Verify certificates were created
ls -la /etc/prosody/certs/

# Set proper ownership and permissions
sudo chown -R prosody:prosody /etc/prosody/certs
sudo chmod 600 /etc/prosody/certs/nto.local.key
sudo chmod 644 /etc/prosody/certs/nto.local.crt
```

### Create Admin User

```bash
# Create admin user for server management
sudo prosodyctl adduser admin@nto.local
# Enter password when prompted (use something simple for development)
```

## Development Workflow

### Daily Development Process

#### 1. Start WSL and Prosody

```powershell
# From PowerShell (Windows)
wsl

# Now you're in WSL - start Prosody
sudo systemctl start prosody

# Check if it's running
sudo systemctl status prosody

# View logs (useful for debugging)
sudo tail -f /var/log/prosody/prosody.log
```

#### 2. Configure Windows Host Resolution

Add to Windows hosts file (`C:\Windows\System32\drivers\etc\hosts`):

```
127.0.0.1    nto.local
127.0.0.1    groups.nto.local
```

**Note**: You need administrator privileges to edit the hosts file.

#### 3. Start NTO Development

```powershell
# In your project directory (new PowerShell window)
cd e:\dev\nto
cargo tauri dev
```

### Port Forwarding (Usually Automatic in WSL 2)

WSL 2 automatically forwards ports from localhost. Your Windows application can connect to:
- `localhost:5222` (XMPP client connections)
- `localhost:5280` (HTTP admin interface)
- `localhost:5281` (HTTPS admin interface)

### Testing the Setup

#### Test XMPP Connection

```bash
# Inside WSL, test local connection
telnet localhost 5222

# You should see XML stream start
# Press Ctrl+] then 'quit' to exit
```

#### Test from Windows

```powershell
# Test port accessibility from Windows
Test-NetConnection -ComputerName localhost -Port 5222
```

## Integration with NTO

### Rust XMPP Client Configuration

In your `src-tauri/src/main.rs`, configure the XMPP client:

```rust
use tokio_xmpp::{Client, Event as XmppEvent};
use xmpp_parsers::{Jid};

// XMPP connection settings for development
const XMPP_SERVER: &str = "nto.local";
const XMPP_PORT: u16 = 5222;

#[command]
async fn connect_xmpp(jid: String, password: String) -> Result<String, String> {
    let jid: Jid = jid.parse().map_err(|e| format!("Invalid JID: {}", e))?;
    
    // Connect to local Prosody server
    let mut client = Client::new(jid, password).await
        .map_err(|e| format!("Failed to connect: {}", e))?;
    
    // Start event loop
    while let Some(event) = client.next().await {
        match event {
            XmppEvent::Online => {
                return Ok("Connected successfully".to_string());
            }
            XmppEvent::Disconnected => {
                return Err("Disconnected".to_string());
            }
            _ => {}
        }
    }
    
    Err("Connection failed".to_string())
}
```

### Test Users for Development

```bash
# Create test users in WSL
sudo prosodyctl adduser alice@nto.local
sudo prosodyctl adduser bob@nto.local
sudo prosodyctl adduser charlie@nto.local
```

## Management and Debugging

### Prosody Management Commands

```bash
# Start/stop/restart Prosody
sudo systemctl start prosody
sudo systemctl stop prosody
sudo systemctl restart prosody

# Enable/disable autostart
sudo systemctl enable prosody
sudo systemctl disable prosody

# View status
sudo systemctl status prosody

# View logs
sudo tail -f /var/log/prosody/prosody.log
sudo tail -f /var/log/prosody/prosody.err

# User management
sudo prosodyctl adduser user@nto.local
sudo prosodyctl deluser user@nto.local
sudo prosodyctl passwd user@nto.local

# Server management
sudo prosodyctl check config
sudo prosodyctl check dns
sudo prosodyctl reload
```

### Development Tips

#### 1. Quick Restart Script

Create `/home/[username]/restart-prosody.sh`:

```bash
#!/bin/bash
echo "Restarting Prosody..."
sudo systemctl restart prosody
echo "Prosody restarted!"
sudo systemctl status prosody
```

Make it executable: `chmod +x restart-prosody.sh`

#### 2. Log Monitoring

```bash
# Watch all logs in real-time
sudo tail -f /var/log/prosody/prosody.log /var/log/prosody/prosody.err

# Filter specific events
sudo grep -i "error\|warning" /var/log/prosody/prosody.log
```

#### 3. Configuration Testing

```bash
# Test configuration without restarting
sudo prosodyctl check config

# Test DNS resolution
sudo prosodyctl check dns nto.local
```

## Troubleshooting

### Common Issues and Solutions

#### 1. "Connection Refused" from Windows App

**Problem**: Your Rust app can't connect to WSL Prosody
**Solutions**:
```powershell
# Check if port is accessible
Test-NetConnection -ComputerName localhost -Port 5222

# Check WSL networking
wsl hostname -I

# Restart WSL networking
wsl --shutdown
wsl
```

#### 2. SSL/TLS Certificate Errors

**Problem**: Certificate validation fails
**Solution**: For development, disable certificate validation in your Rust client:

```rust
// In your XMPP client configuration
client.set_check_certificate(false); // Development only!
```

#### 3. Prosody Won't Start

**Problem**: Service fails to start
**Debugging**:
```bash
# Check configuration
sudo prosodyctl check config

# Check ports
sudo netstat -tlnp | grep :5222

# Check logs
sudo journalctl -u prosody.service -f
```

#### 4. Users Can't Register

**Problem**: Registration fails
**Solution**: Ensure `allow_registration = true` in config and:
```bash
# Check module is loaded
sudo prosodyctl check config | grep register
```

#### 5. WSL Network Issues

**Problem**: Ports not forwarding properly
**Solutions**:
```powershell
# Check WSL version (should be WSL 2)
wsl --list --verbose

# Reset WSL networking
wsl --shutdown
wsl
```

## Advanced Configuration

### Production-Like Setup (Optional)

For testing production scenarios:

#### External Database (PostgreSQL)

```bash
# Install PostgreSQL in WSL
sudo apt install postgresql postgresql-contrib

# Configure Prosody to use PostgreSQL
# Add to prosody.cfg.lua:
```

```lua
sql = {
    driver = "PostgreSQL";
    database = "prosody";
    username = "prosody";
    password = "password";
    host = "localhost";
}

storage = "sql"
```

#### LDAP Authentication

```lua
authentication = "ldap"
ldap = {
    hostname = "localhost";
    bind_dn = "cn=admin,dc=nto,dc=local";
    bind_password = "admin";
    user = {
        basedn = "ou=users,dc=nto,dc=local";
        filter = "(uid={user})";
        usernamefield = "uid";
    }
}
```

## Development Scripts

### PowerShell Helper Script

Create `scripts/start-xmpp-dev.ps1`:

```powershell
# NTO XMPP Development Starter
Write-Host "🚀 Starting XMPP Development Environment" -ForegroundColor Green

# Start WSL and Prosody
Write-Host "📡 Starting Prosody server..." -ForegroundColor Cyan
wsl sudo systemctl start prosody

# Wait a moment for startup
Start-Sleep -Seconds 2

# Check if Prosody is running
$prosodyStatus = wsl sudo systemctl is-active prosody
if ($prosodyStatus -eq "active") {
    Write-Host "✅ Prosody server is running" -ForegroundColor Green
    
    # Test connection
    $connection = Test-NetConnection -ComputerName localhost -Port 5222 -WarningAction SilentlyContinue
    if ($connection.TcpTestSucceeded) {
        Write-Host "✅ XMPP port 5222 is accessible" -ForegroundColor Green
        
        # Start the application
        Write-Host "🎮 Starting NTO..." -ForegroundColor Cyan
        cargo tauri dev
    } else {
        Write-Host "❌ Cannot connect to XMPP port 5222" -ForegroundColor Red
        Write-Host "💡 Try: wsl --shutdown, then run this script again" -ForegroundColor Yellow
    }
} else {
    Write-Host "❌ Prosody failed to start" -ForegroundColor Red
    Write-Host "💡 Check logs with: wsl sudo journalctl -u prosody.service" -ForegroundColor Yellow
}
```

### WSL Helper Script

Create `/home/[username]/emi-dev.sh`:

```bash
#!/bin/bash
# NTO development helper

case "$1" in
    start)
        echo "Starting Prosody..."
        sudo systemctl start prosody
        sudo systemctl status prosody
        ;;
    stop)
        echo "Stopping Prosody..."
        sudo systemctl stop prosody
        ;;
    restart)
        echo "Restarting Prosody..."
        sudo systemctl restart prosody
        sudo systemctl status prosody
        ;;
    logs)
        echo "Showing Prosody logs (Ctrl+C to exit)..."
        sudo tail -f /var/log/prosody/prosody.log
        ;;
    users)
        echo "Current users:"
        sudo prosodyctl list localhost
        ;;
    adduser)
        if [ -z "$2" ]; then
            echo "Usage: $0 adduser username"
            echo "Will create username@nto.local"
        else
            sudo prosodyctl adduser "$2@nto.local"
        fi
        ;;
    *)
        echo "Usage: $0 {start|stop|restart|logs|users|adduser}"
        ;;
esac
```

Make executable: `chmod +x emi-dev.sh`

## Summary

This setup gives you a complete XMPP development environment where:

1. **Prosody runs in WSL** (`localhost:5222`)
2. **Your Windows app connects** to the WSL server
3. **Configuration is in** `/etc/prosody/prosody.cfg.lua` (WSL)
4. **Management is via** `prosodyctl` commands (WSL)
5. **Development workflow** is streamlined with helper scripts

The beauty of this setup is that it closely mimics a production environment while keeping everything local for development. Your Rust XMPP client will work exactly the same way against a remote server in production.

Would you like me to create any specific helper scripts or explain any part of this workflow in more detail?
