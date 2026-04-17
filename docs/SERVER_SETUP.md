# NTO Server Setup — Oracle Cloud Free Tier

Step-by-step guide to provisioning a permanent, zero-cost XMPP server for NTO using Oracle Cloud's Always Free ARM tier.

---

## 1. Oracle Cloud Account

1. Sign up at [cloud.oracle.com](https://cloud.oracle.com).
2. **Choose your home region carefully** — it cannot be changed later. Pick the region geographically closest to you.
3. Verify your account (credit card required for identity, but will not be charged for Always Free resources).

---

## 2. Provision the VM

1. Go to **Compute → Instances → Create Instance**.
2. Name: `nto-xmpp` (or anything you like).
3. Image: **Ubuntu 22.04 LTS** (Canonical). Make sure it says "Always Free-eligible".
4. Shape: Click **Change Shape** → Ampere → **VM.Standard.A1.Flex** → set **4 OCPUs, 24 GB RAM**.
5. Networking: leave defaults (a new VCN will be created).
6. SSH keys: **Generate a key pair**, download both files. You will need the private key to SSH in.
7. Click **Create**. The instance will be `RUNNING` in ~2 minutes.
8. Note the **Public IP address** from the instance details page.

---

## 3. Open Firewall Ports

By default Oracle blocks all inbound traffic except SSH.

1. Go to **Networking → Virtual Cloud Networks → your VCN → Security Lists → Default Security List**.
2. Click **Add Ingress Rules** and add:

| Source CIDR  | Protocol | Destination Port | Description         |
|--------------|----------|------------------|---------------------|
| `0.0.0.0/0`  | TCP      | 5222             | XMPP STARTTLS       |
| `0.0.0.0/0`  | TCP      | 5223             | XMPP direct-TLS     |

SSH (port 22) is already open.

> NTO connects via **direct-TLS on port 5223**. Port 5222 (STARTTLS) is optional but good to have.

Also open the ports in the **Ubuntu firewall** on the VM itself:

```bash
sudo ufw allow 5222/tcp
sudo ufw allow 5223/tcp
sudo ufw reload
```

---

## 4. SSH Into the VM

```bash
ssh -i /path/to/private-key.key ubuntu@<YOUR_PUBLIC_IP>
```

On Windows (PowerShell):

```powershell
ssh -i C:\path\to\privatekey.key ubuntu@<YOUR_PUBLIC_IP>
```

---

## 5. Install Prosody

```bash
sudo apt update && sudo apt upgrade -y
sudo apt install prosody -y
```

---

## 6. Generate a TLS Certificate

For development/personal use, a self-signed certificate is fine. NTO's `native-tls` in dev builds accepts self-signed certs.

```bash
sudo prosodyctl cert generate <YOUR_PUBLIC_IP>
# or if you have a domain:
sudo prosodyctl cert generate your-domain.com
```

For production with a real domain, use Let's Encrypt:

```bash
sudo apt install certbot -y
sudo certbot certonly --standalone -d your-domain.com
# Then point Prosody to /etc/letsencrypt/live/your-domain.com/
```

---

## 7. Configure Prosody

Edit `/etc/prosody/prosody.cfg.lua`:

```bash
sudo nano /etc/prosody/prosody.cfg.lua
```

Replace or add the VirtualHost block. Minimum viable config:

```lua
-- Use your public IP or domain name here
VirtualHost "YOUR_PUBLIC_IP_OR_DOMAIN"

  ssl = {
    key = "/var/lib/prosody/YOUR_PUBLIC_IP_OR_DOMAIN.key",
    certificate = "/var/lib/prosody/YOUR_PUBLIC_IP_OR_DOMAIN.crt",
  }

modules_enabled = {
  "saslauth",
  "roster",
  "vcard",
  "register",
  "ping",
  "carbons",
  "private",
  "blocklist",
}

-- Allow new registrations (disable after creating your accounts)
allow_registration = true

-- Required for direct-TLS on port 5223
legacy_ssl_ports = { 5223 }
```

Save and restart:

```bash
sudo systemctl restart prosody
sudo systemctl enable prosody  # start on boot
```

Check Prosody is running:

```bash
sudo prosodyctl status
```

---

## 8. Create User Accounts

```bash
sudo prosodyctl register alice YOUR_PUBLIC_IP_OR_DOMAIN yourpassword
sudo prosodyctl register bob   YOUR_PUBLIC_IP_OR_DOMAIN yourpassword
```

NTO login JID format: `alice@YOUR_PUBLIC_IP_OR_DOMAIN`

After creating all accounts, **disable open registration** in `prosody.cfg.lua`:

```lua
allow_registration = false
```

Then restart: `sudo systemctl restart prosody`

---

## 9. Test the Connection

From the NTO login page, enter:
- **JID**: `alice@YOUR_PUBLIC_IP_OR_DOMAIN`
- **Password**: `yourpassword`

If connection fails, check:

```bash
# Prosody logs
sudo journalctl -u prosody -f

# Is port 5223 actually open?
sudo ss -tlnp | grep 5223
```

---

## 10. Keepalive Cron (mandatory)

Oracle reclaims VMs where CPU + network + memory all stay below 20% for **7 consecutive days**.

```bash
sudo nano /etc/cron.d/oci-keepalive
```

```
0 */6 * * * root dd if=/dev/urandom of=/tmp/kv bs=1M count=50 2>/dev/null && rm /tmp/kv
```

This writes 50 MB of random data to disk every 6 hours, which is enough to keep metrics above the threshold.

---

## 11. Backup Prosody to OCI Object Storage (optional but recommended)

OCI Object Storage gives **20 GB free** and is independent infrastructure — it survives even if your VM is deleted.

### Set up OCI CLI

```bash
sudo apt install python3-pip -y
pip3 install oci-cli
oci setup config  # follow the prompts, enter your tenancy/user OCIDs from the OCI console
```

### Create an Object Storage bucket

In the OCI console: **Storage → Object Storage → Create Bucket** → name it `prosody-backups`.

### Backup script

```bash
sudo nano /usr/local/bin/backup-prosody.sh
```

```bash
#!/bin/bash
DATE=$(date +%Y%m%d)
BACKUP=/tmp/prosody-$DATE.tar.gz
tar czf $BACKUP /var/lib/prosody/
oci os object put \
  --bucket-name prosody-backups \
  --file $BACKUP \
  --name prosody-$DATE.tar.gz \
  --force
rm $BACKUP
```

```bash
sudo chmod +x /usr/local/bin/backup-prosody.sh
```

Schedule daily at 3 AM:

```bash
sudo nano /etc/cron.d/prosody-backup
```

```
0 3 * * * root /usr/local/bin/backup-prosody.sh
```

### Restore after VM loss

```bash
# 1. Provision a new free ARM VM (steps 1–7 above)
# 2. Download the latest backup:
oci os object get \
  --bucket-name prosody-backups \
  --name prosody-YYYYMMDD.tar.gz \
  --file /tmp/restore.tar.gz
# 3. Restore:
sudo tar xzf /tmp/restore.tar.gz -C /
sudo chown -R prosody:prosody /var/lib/prosody
sudo systemctl restart prosody
```

Maximum data loss with daily backups: 24 hours.

---

## Quick Reference

| Item | Value |
|------|-------|
| XMPP port (direct-TLS) | **5223** |
| XMPP port (STARTTLS) | 5222 |
| JID format | `user@YOUR_IP_OR_DOMAIN` |
| Prosody config | `/etc/prosody/prosody.cfg.lua` |
| Prosody data | `/var/lib/prosody/` |
| Prosody logs | `journalctl -u prosody -f` |
| Keepalive cron | `/etc/cron.d/oci-keepalive` |
| Backup cron | `/etc/cron.d/prosody-backup` |
