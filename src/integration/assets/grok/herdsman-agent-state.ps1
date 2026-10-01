# installed by herdsman
# managed by herdsman; reinstalling or updating the integration overwrites this file.
# add custom hooks beside this file instead of editing it.
# HERDSMAN_INTEGRATION_ID=grok
# HERDSMAN_INTEGRATION_VERSION=2

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

$event = if ($null -ne $payload -and $payload.hook_event_name -is [string]) {
    $payload.hook_event_name
} elseif ($null -ne $payload -and $payload.hookEventName -is [string]) {
    $payload.hookEventName
} else {
    $null
}
if ($null -ne $event -and $event -notin @("session_start", "SessionStart", "sessionStart")) { exit 0 }

$sessionStartSource = if ($null -ne $payload -and $payload.source -is [string]) {
    $payload.source
} else {
    $null
}

$sessionId = $env:GROK_SESSION_ID
if ([string]::IsNullOrWhiteSpace($sessionId) -and $null -ne $payload) {
    if ($payload.session_id -is [string]) { $sessionId = $payload.session_id }
    elseif ($payload.sessionId -is [string]) { $sessionId = $payload.sessionId }
}
if ([string]::IsNullOrWhiteSpace($sessionId)) { exit 0 }

$seq = [DateTime]::UtcNow.Ticks
$herdsman = if ([string]::IsNullOrWhiteSpace($env:HERDSMAN_BIN_PATH)) { "herdsman" } else { $env:HERDSMAN_BIN_PATH }
$herdsmanArgs = @(
    "pane", "report-agent-session", $env:HERDSMAN_PANE_ID,
    "--source", "herdsman:grok",
    "--agent", "grok",
    "--seq", "$seq",
    "--agent-session-id", "$sessionId"
)
if (-not [string]::IsNullOrWhiteSpace($sessionStartSource)) {
    $herdsmanArgs += @("--session-start-source", "$sessionStartSource")
}
try {
    & $herdsman @herdsmanArgs 2>$null | Out-Null
} catch {
}
