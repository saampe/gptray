[CmdletBinding()]
param(
    [string]$Executable = (Join-Path $PSScriptRoot '../src-tauri/target/x86_64-pc-windows-msvc/debug/examples/native_smoke.exe'),
    [string[]]$Arguments = @(),
    [string]$OutputDirectory = '.'
)
# Explicit Process API retains the exit handle and drains redirected output.
# PowerShell 7 is used by CI; this harness never starts the personal GPTray profile.
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force $OutputDirectory | Out-Null
$start = [System.Diagnostics.ProcessStartInfo]::new()
$start.FileName = (Resolve-Path -LiteralPath $Executable).Path
$start.UseShellExecute = $false
$start.RedirectStandardOutput = $true
$start.RedirectStandardError = $true
foreach ($argument in $Arguments) { $start.ArgumentList.Add($argument) }
$probe = [System.Diagnostics.Process]::new()
$probe.StartInfo = $start
$null = $probe.Start()
$stdoutTask = $probe.StandardOutput.ReadToEndAsync()
$stderrTask = $probe.StandardError.ReadToEndAsync()
$timedOut = !$probe.WaitForExit(60000)
if ($timedOut) { $probe.Kill($true) }
$probe.WaitForExit()
$stdout = $stdoutTask.GetAwaiter().GetResult()
$stderr = $stderrTask.GetAwaiter().GetResult()
$exitCode = if ($timedOut) { 124 } else { $probe.ExitCode }
$probe.Dispose()
[System.IO.File]::WriteAllText((Join-Path (Resolve-Path $OutputDirectory).Path 'native-smoke.stdout.txt'), $stdout)
[System.IO.File]::WriteAllText((Join-Path (Resolve-Path $OutputDirectory).Path 'native-smoke.stderr.txt'), $stderr)
$passed = !$timedOut -and $exitCode -eq 0 -and $stdout -match 'NATIVE_SMOKE_PASS:'
$result = [pscustomobject]@{ Passed = $passed; ExitCode = $exitCode; TimedOut = $timedOut;
    Scope = 'Isolated local HTML/native WebView2 API and viewport; not ChatGPT account, rendering quality or RAM measurement';
    UTC = [datetime]::UtcNow.ToString('o') }
$result | ConvertTo-Json | Set-Content (Join-Path $OutputDirectory 'native-smoke-result.json') -Encoding utf8
Write-Output ($result | ConvertTo-Json -Compress)
Write-Output $stdout
Write-Output $stderr
if (!$passed) { throw "Native WebView2 smoke failed (exit $exitCode); see captured logs." }
