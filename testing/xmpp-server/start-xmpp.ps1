# NTO Local XMPP Server
# Starts ejabberd and installs its self-signed cert into the Windows trust store

param(
    [switch]$Stop,
    [switch]$Logs,
    [switch]$Reset
)

$composeFile = "$PSScriptRoot\docker-compose.yml"
$containerName = "nto-xmpp"

if ($Stop) {
    Write-Host "Stopping XMPP server..." -ForegroundColor Yellow
    docker compose -f $composeFile down
    exit 0
}

if ($Logs) {
    docker logs -f $containerName
    exit 0
}

if ($Reset) {
    Write-Host "Removing XMPP server and all data..." -ForegroundColor Red
    docker compose -f $composeFile down -v
    exit 0
}

# --- Start ---
Write-Host ""
Write-Host "Starting NTO local XMPP server..." -ForegroundColor Cyan
docker compose -f $composeFile up -d

if ($LASTEXITCODE -ne 0) {
    Write-Host "Docker compose failed. Is Docker Desktop running?" -ForegroundColor Red
    exit 1
}

# Wait for ejabberd to be ready
Write-Host "Waiting for ejabberd to start..." -ForegroundColor Yellow
$ready = $false
for ($i = 0; $i -lt 30; $i++) {
    Start-Sleep -Seconds 2
    $status = docker exec $containerName ejabberdctl status 2>&1
    if ($status -match "is running") {
        $ready = $true
        break
    }
    Write-Host "  Still starting... ($($i * 2)s)" -ForegroundColor Gray
}

if (-not $ready) {
    Write-Host "ejabberd did not start in time. Check logs:" -ForegroundColor Red
    Write-Host "  .\start-xmpp.ps1 -Logs" -ForegroundColor Gray
    exit 1
}

Write-Host "ejabberd is running." -ForegroundColor Green

# --- Trust the self-signed certificate ---
Write-Host ""
Write-Host "Installing ejabberd self-signed cert into Windows trust store..." -ForegroundColor Cyan

$certPath = "$env:TEMP\ejabberd-localhost.pem"
$cerPath  = "$env:TEMP\ejabberd-localhost.cer"
docker exec $containerName cat /home/ejabberd/conf/certs/localhost.pem | Out-File -Encoding ascii $certPath
# combined PEM = private key then certificate — extract the cert block only
if (Test-Path $certPath) {
    $lines = Get-Content $certPath
    $certStart = ($lines | Select-String -Pattern "BEGIN CERTIFICATE").LineNumber
    if ($certStart) {
        $lines | Select-Object -Skip ($certStart - 1) | Out-File -Encoding ascii $cerPath
    }
}

if (Test-Path $cerPath) {
    # Check if already trusted
    $existing = Get-ChildItem Cert:\LocalMachine\Root | Where-Object { $_.Subject -match "localhost" }
    if ($existing) {
        Write-Host "Certificate already trusted." -ForegroundColor Gray
    } else {
        try {
            Import-Certificate -FilePath $cerPath -CertStoreLocation Cert:\LocalMachine\Root | Out-Null
            Write-Host "Certificate trusted successfully." -ForegroundColor Green
        } catch {
            Write-Host "Could not auto-install certificate (need to run as Administrator)." -ForegroundColor Yellow
            Write-Host "Run this script as Administrator, or run manually:" -ForegroundColor Yellow
            Write-Host "  Import-Certificate -FilePath '$cerPath' -CertStoreLocation Cert:\LocalMachine\Root" -ForegroundColor Gray
        }
    }
    Remove-Item $certPath -Force -ErrorAction SilentlyContinue
} else {
    Write-Host "Could not extract certificate from container." -ForegroundColor Yellow
}

# --- Create a default test account ---
Write-Host ""
Write-Host "Creating test account testuser@localhost..." -ForegroundColor Cyan
$regResult = docker exec $containerName ejabberdctl register testuser localhost testpassword 2>&1
if ($regResult -match "already registered") {
    Write-Host "testuser@localhost already exists." -ForegroundColor Gray
} elseif ($regResult -match "User testuser@localhost successfully registered") {
    Write-Host "Created: testuser@localhost / testpassword" -ForegroundColor Green
} else {
    Write-Host "  $regResult" -ForegroundColor Gray
}

# --- Create friend accounts (password: password123) ---
Write-Host ""
Write-Host "Creating friend accounts..." -ForegroundColor Cyan

$friendAccounts = @(
    @{ user = "juniorbcm";       name = "Death Scyther" },
    @{ user = "giovanni_p";      name = "Burega The King" },
    @{ user = "galaxyblues";     name = "Sawamura Shido" },
    @{ user = "ivokds";          name = "Kushirenada" },
    @{ user = "ishiro_oninawa";  name = "Ishiro Oninawa" },
    @{ user = "igorcds";         name = "Oniguma" },
    @{ user = "rafaelsbz";       name = "Fael" },
    @{ user = "kampelo";         name = "Kampelo" },
    @{ user = "sancho_p";        name = "Sancho" },
    @{ user = "mateusbrigido";   name = "Hanatarou" },
    @{ user = "funboy";          name = "Fun Boy" },
    @{ user = "anderson";        name = "Anderson" },
    @{ user = "netokbca";        name = "Kbca" },
    @{ user = "abrupt";          name = "Abrupt Chemical Mind" },
    @{ user = "kkxi";            name = "Kakashi Sahringam" },
    @{ user = "benjin";          name = "Benji Maden" },
    @{ user = "mastrangelo";     name = "V.A.M." },
    @{ user = "sephirothx";      name = "SephirothX" },
    @{ user = "thelast";         name = "The Last" },
    @{ user = "teresaarg";       name = "Undomiel Pastel" },
    @{ user = "ayres";           name = "Bloody George" },
    @{ user = "giuliano";        name = "Giuliano" },
    @{ user = "msoares";         name = "Marcelo" },
    @{ user = "anandaomati";     name = "Ananda" },
    @{ user = "relouin";         name = "Relouin" },
    @{ user = "aav";             name = "Abel" },
    @{ user = "lav";             name = "Luna" },
    @{ user = "perhaps";         name = "Deborah" },
    @{ user = "lary_zv";         name = "Laryssa" },
    @{ user = "l_over";          name = "Mandy" }
)

foreach ($acct in $friendAccounts) {
    $r = docker exec $containerName ejabberdctl register $acct.user localhost password123 2>&1
    if ($r -match "already registered") {
        Write-Host "  $($acct.user)@localhost already exists." -ForegroundColor Gray
    } elseif ($r -match "successfully registered") {
        Write-Host "  Created: $($acct.user)@localhost  ($($acct.name))" -ForegroundColor Green
    } else {
        Write-Host "  $($acct.user): $r" -ForegroundColor Yellow
    }
}

# --- Summary ---
Write-Host ""
Write-Host "====================================================" -ForegroundColor Green
Write-Host " XMPP server ready for NTO testing" -ForegroundColor Green
Write-Host "====================================================" -ForegroundColor Green
Write-Host " Admin panel : http://localhost:5280/admin"
Write-Host "              (login: admin@localhost / admin)"
Write-Host " Test account: testuser@localhost / testpassword"
Write-Host " Direct-TLS  : localhost:5223"
Write-Host ""
Write-Host " In NTO — use 'testuser@localhost' as the JID"
Write-Host ""
Write-Host " Stop : .\start-xmpp.ps1 -Stop"
Write-Host " Logs : .\start-xmpp.ps1 -Logs"
Write-Host " Reset: .\start-xmpp.ps1 -Reset  (wipes all data)"
Write-Host "====================================================" -ForegroundColor Green
