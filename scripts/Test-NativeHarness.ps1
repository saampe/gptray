# Exercise the real child-process harness using deterministic local subprocesses.
$ErrorActionPreference = 'Stop'
$root = Join-Path ([System.IO.Path]::GetTempPath()) ([guid]::NewGuid().ToString())
$pwshPath = (Get-Process -Id $PID).Path
try {
    & "$PSScriptRoot/Test-NativeWebView.ps1" -Executable $pwshPath -Arguments @('-NoProfile', '-Command', "Write-Output 'NATIVE_SMOKE_PASS: fixture'; exit 0") -OutputDirectory (Join-Path $root 'pass') | Out-Null
    $pass = Get-Content (Join-Path $root 'pass/native-smoke-result.json') -Raw | ConvertFrom-Json
    if (!$pass.Passed -or $pass.ExitCode -ne 0) { throw 'Successful subprocess was lost' }
    foreach ($case in @(@('missing-marker', 'exit 0', 0), @('native-error', "[Console]::Error.WriteLine('fixture failure'); exit 7", 7))) {
        $failed = $false
        try { & "$PSScriptRoot/Test-NativeWebView.ps1" -Executable $pwshPath -Arguments @('-NoProfile', '-Command', $case[1]) -OutputDirectory (Join-Path $root $case[0]) | Out-Null } catch { $failed = $true }
        $result = Get-Content (Join-Path $root "$($case[0])/native-smoke-result.json") -Raw | ConvertFrom-Json
        if (!$failed -or $result.Passed -or $result.ExitCode -ne $case[2]) { throw "Harness failed to reject $($case[0])" }
    }
    $stderr = Get-Content (Join-Path $root 'native-error/native-smoke.stderr.txt') -Raw
    if ($stderr -notmatch 'fixture failure') { throw 'Child stderr was lost' }
    Write-Output 'Native process harness: 3 subprocess scenarios passed.'
} finally { if (Test-Path $root) { Remove-Item -Recurse -Force -LiteralPath $root } }
