# Run the compiled probe with an external timeout, including setup/runtime hangs.
$ErrorActionPreference = 'Stop'
$executable = Join-Path $PSScriptRoot '../src-tauri/target/x86_64-pc-windows-msvc/debug/examples/native_smoke.exe'
$probe = Start-Process -FilePath $executable -PassThru -RedirectStandardOutput 'native-smoke.stdout.txt' -RedirectStandardError 'native-smoke.stderr.txt'
$timedOut = !$probe.WaitForExit(60000)
if ($timedOut) { Stop-Process -Id $probe.Id -Force -ErrorAction SilentlyContinue }
$probe.Refresh()
$exitCode = if ($timedOut) { 124 } else { $probe.ExitCode }
$stdout = Get-Content 'native-smoke.stdout.txt' -Raw -ErrorAction SilentlyContinue
$stderr = Get-Content 'native-smoke.stderr.txt' -Raw -ErrorAction SilentlyContinue
$passed = !$timedOut -and $exitCode -eq 0 -and $stdout -match 'NATIVE_SMOKE_PASS:'
[pscustomobject]@{ Passed = $passed; ExitCode = $exitCode; TimedOut = $timedOut;
    Scope = 'Isolated local HTML/native WebView2 API and viewport; not ChatGPT account, rendering quality or RAM measurement';
    UTC = [datetime]::UtcNow.ToString('o') } | ConvertTo-Json | Set-Content 'native-smoke-result.json' -Encoding utf8
Write-Output $stdout
Write-Output $stderr
if (!$passed) { throw 'Native WebView2 smoke probe failed; see captured logs.' }
