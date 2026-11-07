[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateRange(1, 2147483647)][int]$RootProcessId,
    [Parameter(Mandatory)][ValidateNotNullOrEmpty()][string]$Scenario,
    [ValidateRange(1, 3600)][int]$Samples = 30,
    [ValidateRange(1, 60)][int]$IntervalSeconds = 1,
    [string]$ProfileDirectory = (Join-Path $env:LOCALAPPDATA 'com.globalvox.chatgpt-overlay-bar'),
    [string]$OutputDirectory = '.\memory-results'
)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/ProcessGroup.ps1"
New-Item -ItemType Directory -Force $OutputDirectory | Out-Null
# Fail clearly for an incorrect PID rather than silently reporting an empty baseline.
$root = Get-CimInstance Win32_Process -Filter "ProcessId=$RootProcessId"
if (!$root -or $root.Name -ine 'chatgpt-overlay-bar.exe') { throw 'Select the GPTray host PID (chatgpt-overlay-bar.exe).' }
$known = @{ RootIdentity = ('{0}:{1}' -f $RootProcessId, ([datetime]$root.CreationDate).ToUniversalTime().Ticks) }
$rows = [System.Collections.Generic.List[object]]::new()
$totals = [System.Collections.Generic.List[object]]::new()
for ($sample = 0; $sample -lt $Samples; $sample++) {
    $timestamp = [datetime]::UtcNow.ToString('o')
    $snapshot = @(Get-CimInstance Win32_Process)
    $group = @(Get-GPTrayProcessGroup $snapshot $RootProcessId $known $ProfileDirectory)
    $perfById = @{}
    foreach ($counter in (Get-CimInstance Win32_PerfRawData_PerfProc_Process)) {
        if ($counter.IDProcess -gt 0) { $perfById[[int]$counter.IDProcess] = $counter }
    }
    [long]$privateWorkingSet = 0; [long]$privateBytes = 0; [long]$workingSet = 0
    $hostPresent = Test-GPTrayRootPresent $snapshot $RootProcessId $known
    $complete = $hostPresent
    foreach ($process in $group) {
        $id = [int]$process.ProcessId
        try {
            $live = Get-Process -Id $id -ErrorAction Stop
            if (!(Test-GPTraySameStartTime ([datetime]$process.CreationDate) $live.StartTime)) {
                throw 'PID recycled during sample'
            }
            if (!$perfById.ContainsKey($id)) { throw 'No private working-set counter' }
            $counter = $perfById[$id]
            $privateWorkingSet += [long]$counter.WorkingSetPrivate
            $privateBytes += $live.PrivateMemorySize64
            $workingSet += $live.WorkingSet64
            $rows.Add([pscustomobject]@{ UTC = $timestamp; Sample = $sample; Scenario = $Scenario;
                ProcessId = $id; ParentProcessId = $process.ParentProcessId; Name = $process.Name;
                CreatedUTC = ([datetime]$process.CreationDate).ToUniversalTime().ToString('o');
                PrivateWorkingSetBytes = [long]$counter.WorkingSetPrivate;
                PrivateBytes = $live.PrivateMemorySize64; WorkingSetBytes = $live.WorkingSet64 })
        } catch { $complete = $false; Write-Warning "Sample $sample incomplete: process $id exited or a counter was unavailable." }
    }
    $totals.Add([pscustomobject]@{ UTC = $timestamp; Sample = $sample; Scenario = $Scenario;
        Complete = $complete; HostPresent = $hostPresent; ProcessCount = $group.Count; PrivateWorkingSetMiB = $privateWorkingSet / 1MB;
        PrivateCommitMiB = $privateBytes / 1MB; WorkingSetMiB = $workingSet / 1MB })
    if ($sample -lt $Samples - 1) { Start-Sleep -Seconds $IntervalSeconds }
}
$stamp = [datetime]::UtcNow.ToString('yyyyMMdd-HHmmss-fff')
$rows | Export-Csv (Join-Path $OutputDirectory "$stamp-processes.csv") -NoTypeInformation -Encoding utf8
$totals | Export-Csv (Join-Path $OutputDirectory "$stamp-totals.csv") -NoTypeInformation -Encoding utf8
$valid = @($totals | Where-Object Complete)
if ($valid.Count) { $valid | Measure-Object PrivateWorkingSetMiB, PrivateCommitMiB, WorkingSetMiB -Average -Minimum -Maximum }
else { Write-Warning 'No complete samples: inspect warnings and retry.' }
Write-Output "Saved $($totals.Count) samples ($($valid.Count) complete) in $OutputDirectory"
