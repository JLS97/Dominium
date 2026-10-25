# Hook SubagentStop: cuando un implementador (o el investigador de rendimiento)
# da por terminada su tarea, comprueba en su worktree que el workspace compila
# y que pasa `cargo xtask check-arch`. Si no, le devuelve el error para que
# siga trabajando.
#
#   - Lee la ruta del worktree de la linea "Worktree: <ruta>" del informe final.
#   - No comprueba nada si el informe dice "Estado: bloqueado".
#   - Bloquea como mucho dos veces por subagente (contador en %TEMP%), para no
#     entrar en bucle.
#   - Los tests completos y el resto de la puerta los pasan /verificar e /integrar.
#
# Sale con 0 para dejar terminar y con 2 (motivo en stderr) para que siga.
# Archivo en ASCII a proposito (Windows PowerShell 5.1 y los .ps1 sin BOM).

$ErrorActionPreference = 'Stop'
$MaxBlocks = 2
$Utf8 = New-Object System.Text.UTF8Encoding $false

function Block([string]$reason) {
    $err = New-Object System.IO.StreamWriter([Console]::OpenStandardError(), $Utf8)
    $err.WriteLine("Comprobacion al terminar (.claude/hooks/comprobar-al-terminar.ps1): $reason")
    $err.Flush()
    exit 2
}

try {
    $reader = New-Object System.IO.StreamReader([Console]::OpenStandardInput(), $Utf8)
    $raw = $reader.ReadToEnd()
    $data = $raw | ConvertFrom-Json
} catch {
    exit 0
}

$agent = [string]$data.agent_type
if ([string]::IsNullOrEmpty($agent)) { exit 0 }
$agent = ($agent -split ':')[-1].ToLowerInvariant()
if (-not (@('implementador', 'investigador-rendimiento') -contains $agent)) { exit 0 }

# Guarda propia contra bucles: un contador por subagente en %TEMP%.
$id = [string]$data.agent_id
if ([string]::IsNullOrEmpty($id)) { $id = [string]$data.session_id + '-' + $agent }
$id = $id -replace '[^A-Za-z0-9_.-]', '_'
$counter = Join-Path ([System.IO.Path]::GetTempPath()) ("plus-ultra-stop-" + $id + ".txt")
$count = 0
if (Test-Path -LiteralPath $counter) {
    try { $count = [int](Get-Content -LiteralPath $counter -Raw) } catch { $count = 0 }
}
if ($count -ge $MaxBlocks) { exit 0 }

function BlockCounted([string]$reason) {
    Set-Content -LiteralPath $counter -Value ($count + 1) -Encoding ASCII
    Block $reason
}

$msg = [string]$data.last_assistant_message
if ([string]::IsNullOrEmpty($msg)) { exit 0 }
if ($msg -match '(?im)^\s*Estado:\s*bloqueado') { exit 0 }

$m = [regex]::Match($msg, '(?im)^[\s>*_-]*Worktree[*_]*:[*_]*\s*`?([^`\r\n]+?)`?[*_]*\s*$')
if (-not $m.Success) {
    BlockCounted "tu informe final no tiene la linea 'Worktree: <ruta absoluta>'. Repite el informe con el formato de tu definicion."
}
$wt = $m.Groups[1].Value.Trim()
# /c/Users/... (estilo Git Bash) -> c:/Users/...
if ($wt -match '^/([a-zA-Z])/(.*)$') { $wt = $Matches[1] + ':/' + $Matches[2] }
if (-not (Test-Path -LiteralPath (Join-Path $wt 'Cargo.toml'))) {
    BlockCounted "la ruta de la linea 'Worktree:' no es un worktree con Cargo.toml: $wt"
}

Set-Location -LiteralPath $wt
$ErrorActionPreference = 'Continue'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

function Run([string]$label, [string[]]$cargoArgs) {
    $out = & cargo @cargoArgs 2>&1 | ForEach-Object { "$_" }
    if ($LASTEXITCODE -ne 0) {
        $tail = ($out | Select-Object -Last 40) -join "`n"
        BlockCounted "'$label' falla en $wt. Corrigelo antes de terminar (o termina con 'Estado: bloqueado' y explica por que).`n$tail"
    }
}

Run 'cargo check --workspace --all-targets' @('check', '--workspace', '--all-targets', '--quiet')
Run 'cargo xtask check-arch' @('xtask', 'check-arch')

if (Test-Path -LiteralPath $counter) { Remove-Item -LiteralPath $counter -ErrorAction SilentlyContinue }
exit 0
