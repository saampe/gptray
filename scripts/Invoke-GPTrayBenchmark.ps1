[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateNotNullOrEmpty()][string]$VersionLabel,
    [ValidateRange(0, 2147483647)][int]$RootProcessId = 0,
    [ValidateRange(1, 300)][int]$SamplesPerScenario = 30,
    [ValidateSet('LoggedOut', 'LoggedIn', 'Unknown')][string]$AuthState = 'Unknown',
    [Nullable[int]]$DisplayScalePercent,
    [string]$OutputDirectory = '.\memory-results'
)
$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') { throw 'This benchmark requires a Windows session running GPTray.' }
if (!$RootProcessId) {
    $hosts = @(Get-Process 'chatgpt-overlay-bar' -ErrorAction SilentlyContinue)
    if ($hosts.Count -ne 1) { throw 'Start exactly one GPTray host without opening it; or supply -RootProcessId.' }
    $RootProcessId = $hosts[0].Id
}
$root = Get-CimInstance Win32_Process -Filter "ProcessId=$RootProcessId"
if (!$root -or $root.Name -ine 'chatgpt-overlay-bar.exe') { throw 'Invalid GPTray host.' }
$runDirectory = Join-Path $OutputDirectory ([datetime]::UtcNow.ToString('yyyyMMdd-HHmmss-fff'))
New-Item -ItemType Directory -Force $runDirectory | Out-Null
$osInfo = Get-CimInstance Win32_OperatingSystem
$computer = Get-CimInstance Win32_ComputerSystem
$version = (Get-Item -LiteralPath $root.ExecutablePath).VersionInfo
if ($version.ProductVersion -and !$version.ProductVersion.StartsWith($VersionLabel)) { throw "Executable version $($version.ProductVersion) does not match label $VersionLabel." }
$metadata = [ordered]@{ VersionLabel = $VersionLabel; ExecutableProductVersion = $version.ProductVersion;
    ExecutableSHA256 = (Get-FileHash -LiteralPath $root.ExecutablePath -Algorithm SHA256).Hash.ToLowerInvariant();
    StartedUTC = [datetime]::UtcNow.ToString('o'); RootProcessId = $RootProcessId; OSBuild = $osInfo.BuildNumber;
    OSVersion = $osInfo.Version; CPU = @((Get-CimInstance Win32_Processor).Name); RAMBytes = $computer.TotalPhysicalMemory;
    AuthState = $AuthState; DisplayScalePercent = $DisplayScalePercent; SamplesPerScenario = $SamplesPerScenario;
    PrimaryMetric = 'PrivateWorkingSetMiB'; MiBBytes = 1048576;
    RuntimeVersions = @((Get-Process msedgewebview2 -ErrorAction SilentlyContinue | ForEach-Object {
        try { $_.MainModule.FileVersionInfo.ProductVersion } catch { 'unavailable' }
    }) | Sort-Object -Unique);
    RuntimeVersionNote = 'Installed/running machine versions; per-process attribution is in CSV, not all WebViews are included in RAM totals.' }
$metadata | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $runDirectory 'metadata.json') -Encoding utf8
$scenarios = @(
    @('01-tray-only', 'Fresh GPTray, NEVER opened. Press Enter to capture.'),
    @('02-opening-transition', 'Press Enter to START capture, then immediately open Full via tray. First rows may be tray-only: inspect the time series and record the opening time; this phase is not a steady-state mean.'),
    @('03-open-idle', 'Open Full via tray (works in both versions). Leave the same page idle; press Enter then refocus GPTray during the 5-second countdown.'),
    @('04-hidden-preserved', 'Hide GPTray using native close or Quick blur. Keep auto-discard OFF; press Enter.'),
    @('05-reopened', 'Reopen Full via tray. Press Enter then refocus GPTray during the countdown.'),
    @('06-free-ram', 'Use warned Free RAM only with disposable state. Leave GPTray unloaded; press Enter.'),
    @('07-repeated-cycles', 'Perform 20 open/hide and Quick/Full cycles, then leave Full open on the SAME page. Press Enter then refocus it during the countdown.')
)
Write-Host 'Use the same account/page, runtime, DPI, power mode and background workload for both versions. No text or credentials are entered by this script.'
foreach ($scenario in $scenarios) {
    Write-Host "`n$($scenario[0]): $($scenario[1])"
    $null = Read-Host
    if ($scenario[0] -notin @('01-tray-only', '02-opening-transition', '04-hidden-preserved', '06-free-ram')) { Start-Sleep -Seconds 5 }
    & "$PSScriptRoot/Measure-GPTrayMemory.ps1" -RootProcessId $RootProcessId -Scenario $scenario[0] -VersionLabel $VersionLabel -Samples $SamplesPerScenario -OutputDirectory $runDirectory
}
$metadata['AssociatedRuntimeVersions'] = @(Get-ChildItem $runDirectory -Filter '*-processes.csv' | ForEach-Object { Import-Csv $_.FullName } | Where-Object RuntimeVersion | Select-Object -ExpandProperty RuntimeVersion -Unique)
$metadata | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $runDirectory 'metadata.json') -Encoding utf8
. "$PSScriptRoot/BenchmarkSummary.ps1"
$rows = @(Get-ChildItem $runDirectory -Filter '*-totals.csv' | ForEach-Object { Import-Csv $_.FullName })
$summary = @(Get-GPTrayMemorySummary $rows)
$summary | Export-Csv (Join-Path $runDirectory 'summary.csv') -NoTypeInformation -Encoding utf8
$summary | Format-Table VersionLabel, Scenario, CompleteSamples, PrivateWorkingSetMeanMiB, PrivateCommitMeanMiB, WorkingSetMeanMiB, ProcessCountMean
Write-Output "Benchmark saved to $runDirectory. Keep per-sample CSV: first-load peaks are not steady-state results. No 0.2/0.3 saving is computed without matched conditions."
