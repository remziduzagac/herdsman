# installed by herdsman
# managed by herdsman; reinstalling or updating the integration overwrites this file.
# add custom hooks beside this file instead of editing it.
# HERDSMAN_INTEGRATION_ID=qwen
# HERDSMAN_INTEGRATION_VERSION=2

param([string]$Action = "")

if ($Action -ne "session") { exit 0 }
if ($env:HERDSMAN_ENV -ne "1") { exit 0 }
if ([string]::IsNullOrWhiteSpace($env:HERDSMAN_PANE_ID)) { exit 0 }
if ([string]::IsNullOrWhiteSpace($env:HERDSMAN_SOCKET_PATH)) { exit 0 }

# Read hook input as raw bytes so UTF-8 payloads decode correctly even when the
# console input code page is not UTF-8 (for example CP949 on Korean Windows).
$stdinReader = New-Object System.IO.StreamReader([Console]::OpenStandardInput(), [System.Text.Encoding]::UTF8)
$inputText = $stdinReader.ReadToEnd()
try {
    $payload = if ([string]::IsNullOrWhiteSpace($inputText)) { $null } else { $inputText | ConvertFrom-Json }
} catch {
    $payload = $null
}

if ($null -eq $payload -or [string]::IsNullOrWhiteSpace($payload.session_id)) { exit 0 }

$seq = [DateTime]::UtcNow.Ticks
$herdsman = if ([string]::IsNullOrWhiteSpace($env:HERDSMAN_BIN_PATH)) { "herdsman" } else { $env:HERDSMAN_BIN_PATH }
$commandArgs = @(
    "pane", "report-agent-session", $env:HERDSMAN_PANE_ID,
    "--source", "herdsman:qwen", "--agent", "qwen",
    "--agent-session-id", [string]$payload.session_id,
    "--seq", [string]$seq
)
if ($payload.source -in @("startup", "resume", "clear", "compact", "branch")) {
    $commandArgs += @("--session-start-source", [string]$payload.source)
}
try {
    & $herdsman @commandArgs 2>$null | Out-Null
} catch {
}
