# Hook PreToolUse (Edit|Write|MultiEdit|NotebookEdit): limita que puede editar
# cada subagente del flujo multiagente.
#
#   - Subagentes que editan (implementador, autor-tests, investigador-rendimiento,
#     analista-partidas): solo dentro de un worktree de .worktrees/ y nunca
#     specs/, CLAUDE.md, .claude/, rust-toolchain.toml ni deny.toml.
#   - implementador e investigador-rendimiento: tampoco los tests de aceptacion.
#   - autor-tests: solo tests/, benches/ y Cargo.toml de los crates.
#   - analista-partidas: solo archivos .ron.
#   - Revisores y explorador: nada.
#   - Sesion principal y otros agentes: permitido; mandan los permisos de
#     .claude/settings.json.
#
# Sale con 0 para permitir y con 2 (motivo en stderr) para bloquear. Si no puede
# leer la entrada, permite: es una red de seguridad, no la unica comprobacion
# (/integrar revisa el alcance de cada rama).
#
# Archivo en ASCII a proposito: Windows PowerShell 5.1 lee los .ps1 sin BOM
# como ANSI y estropearia las tildes.

$ErrorActionPreference = 'Stop'
$Utf8 = New-Object System.Text.UTF8Encoding $false

function Deny([string]$reason) {
    $err = New-Object System.IO.StreamWriter([Console]::OpenStandardError(), $Utf8)
    $err.WriteLine("Bloqueado por .claude/hooks/guarda-ediciones.ps1: $reason")
    $err.Flush()
    exit 2
}

function Normalize([string]$p) {
    if ([string]::IsNullOrEmpty($p)) { return '' }
    $p = $p -replace '\\', '/'
    # /c/Users/... (estilo Git Bash) -> c:/Users/...
    if ($p -match '^/([a-zA-Z])/(.*)$') { $p = $Matches[1] + ':/' + $Matches[2] }
    return $p.TrimEnd('/')
}

# Lee stdin como UTF-8 sin depender de la codificacion de la consola.
try {
    $reader = New-Object System.IO.StreamReader([Console]::OpenStandardInput(), $Utf8)
    $raw = $reader.ReadToEnd()
} catch {
    exit 0
}

$agent = ''
$path = ''
try {
    $data = $raw | ConvertFrom-Json
    $agent = [string]$data.agent_type
    $path = [string]$data.tool_input.file_path
    if ([string]::IsNullOrEmpty($path)) { $path = [string]$data.tool_input.notebook_path }
} catch {
    # JSON demasiado grande o raro: extrae solo los dos campos que hacen falta.
    $ma = [regex]::Match($raw, '"agent_type"\s*:\s*"([^"]*)"')
    if ($ma.Success) { $agent = $ma.Groups[1].Value }
    $mp = [regex]::Match($raw, '"(?:file_path|notebook_path)"\s*:\s*"((?:[^"\\]|\\.)*)"')
    if ($mp.Success) { $path = $mp.Groups[1].Value -replace '\\\\', '\' }
}

if ([string]::IsNullOrEmpty($agent)) { exit 0 }
# Tolera prefijos de espacio de nombres ("plugin:nombre").
$agent = ($agent -split ':')[-1].ToLowerInvariant()

$readOnly = @('explorador', 'revisor-constitucion', 'cazador-determinismo', 'revisor-spec')
$editors = @('implementador', 'autor-tests', 'investigador-rendimiento', 'analista-partidas')

if ($readOnly -contains $agent) {
    Deny "'$agent' es un rol de solo lectura; devuelve tus propuestas en el informe."
}
if (-not ($editors -contains $agent)) { exit 0 }

if ([string]::IsNullOrEmpty($path)) { exit 0 }
$norm = Normalize $path

# Ruta relativa a la raiz del worktree. Fuera de un worktree no se edita.
$m = [regex]::Match($norm, '(?i)/\.worktrees/([^/]+)/(.+)$')
if (-not $m.Success) {
    Deny "'$agent' solo edita dentro de su worktree (.worktrees/<tarea>/). El checkout principal es del orquestador. Ruta: $path"
}
$rel = $m.Groups[2].Value

# Comun a todos los que editan.
if ($rel -match '(?i)^specs/') {
    Deny "specs/ lo escribe la sesion principal. Si la spec esta mal o falta algo, para y explicalo en el informe."
}
if ($rel -match '(?i)(^|/)CLAUDE\.md$') {
    Deny "los CLAUDE.md los mantiene la sesion principal."
}
if ($rel -match '(?i)^\.claude/') {
    Deny "la configuracion de agentes (.claude/) no se toca desde un subagente."
}
if ($rel -match '(?i)^(rust-toolchain\.toml|deny\.toml)$') {
    Deny "$rel se cambia de forma deliberada, con el autor, no dentro de una tarea."
}

switch ($agent) {
    { $_ -in @('implementador', 'investigador-rendimiento') } {
        if ($rel -match '(?i)(^|/)tests/aceptacion[^/]*$') {
            Deny "los tests de aceptacion los escribe autor-tests y no se modifican. Si crees que uno esta mal, para y explica por que en el informe."
        }
        exit 0
    }
    'autor-tests' {
        $allowed = @(
            '(?i)^(engine/[^/]+|game/[^/]+|xtask)/(tests|benches)/',
            '(?i)^(engine/[^/]+|game/[^/]+|xtask)/Cargo\.toml$'
        )
        foreach ($rx in $allowed) {
            if ($rel -match $rx) { exit 0 }
        }
        Deny "autor-tests solo escribe en <crate>/tests/, <crate>/benches/ y el Cargo.toml de su crate; las dependencias del workspace las pone la tarea de contrato. Ruta: $rel"
    }
    'analista-partidas' {
        if ($rel -match '(?i)\.ron$') { exit 0 }
        Deny "analista-partidas solo cambia datos .ron; lo que necesite codigo va en el informe. Ruta: $rel"
    }
}

exit 0
