# installed by herdsman
# managed by herdsman; reinstalling or updating the integration overwrites this file.
# add custom hooks beside this file instead of editing it.
# HERDSMAN_INTEGRATION_ID=droid
# HERDSMAN_INTEGRATION_VERSION=3

param([string]$Action = "")

if ($Action -ne "session") { exit 0 }
if ($env:HERDSMAN_ENV -ne "1") { exit 0 }
if ([string]::IsNullOrWhiteSpace($env:HERDSMAN_PANE_ID)) { exit 0 }

$inputText = [Console]::In.ReadToEnd()
try {
    $payload = if ([string]::IsNullOrWhiteSpace($inputText)) { $null } else { $inputText | ConvertFrom-Json }
} catch {
    $payload = $null
}

if ($null -eq $payload -or [string]::IsNullOrWhiteSpace($payload.session_id)) { exit 0 }

$seq = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
$herdsman = if ([string]::IsNullOrWhiteSpace($env:HERDSMAN_BIN_PATH)) { "herdsman" } else { $env:HERDSMAN_BIN_PATH }
try {
    & $herdsman pane report-agent-session $env:HERDSMAN_PANE_ID --source herdsman:droid --agent droid --agent-session-id $payload.session_id --seq $seq 2>$null | Out-Null
} catch {
}
