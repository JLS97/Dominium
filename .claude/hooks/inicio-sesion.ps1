# Hook SessionStart: da a la sesion principal un resumen corto del estado del
# repo (rama, cambios sin commit, worktrees de tareas y preguntas pendientes)
# y le recuerda su papel de orquestador. Su salida entra en el contexto.
# Nunca falla: cualquier error se ignora y sale con 0.
# Archivo en ASCII a proposito (Windows PowerShell 5.1 y los .ps1 sin BOM).

$ErrorActionPreference = 'SilentlyContinue'
try {
    if (-not [string]::IsNullOrEmpty($env:CLAUDE_PROJECT_DIR)) {
        Set-Location -LiteralPath $env:CLAUDE_PROJECT_DIR
    }

    $branch = (& git rev-parse --abbrev-ref HEAD 2>$null)
    $dirty = @(& git status --porcelain 2>$null | Where-Object { $_ -notmatch '(^|/)\.worktrees/' }).Count

    $worktrees = @()
    $current = $null
    foreach ($line in @(& git worktree list --porcelain 2>$null)) {
        if ($line -like 'worktree *') { $current = $line.Substring(9) }
        elseif ($line -like 'branch *' -and $current -match '/\.worktrees/') {
            $worktrees += ($line.Substring(7) -replace '^refs/heads/', '') + ' en ' + $current
        }
    }

    $questions = 0
    $qfile = 'specs/preguntas-pendientes.md'
    if (Test-Path -LiteralPath $qfile) {
        $questions = @(Select-String -LiteralPath $qfile -Pattern '^\s*- \[ \]').Count
    }

    $lines = @()
    $lines += 'Contexto de inicio (hook .claude/hooks/inicio-sesion.ps1):'
    $lines += "- Rama actual: $branch; archivos con cambios sin commit: $dirty"
    if ($worktrees.Count -gt 0) {
        $lines += '- Worktrees de tareas abiertos:'
        foreach ($w in $worktrees) { $lines += "  - $w" }
    } else {
        $lines += '- Worktrees de tareas abiertos: ninguno'
    }
    $lines += "- Preguntas pendientes para el autor en specs/preguntas-pendientes.md: $questions"
    $lines += '- Eres el orquestador: sigue la seccion "Flujo multiagente" de CLAUDE.md. Comandos: /estado /spec /planificar /tareas /implementar /verificar /integrar.'
    Write-Output ($lines -join "`n")
} catch {
}
exit 0
