$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/ProcessGroup.ps1"
function Row($id, $parent, $name, $seconds, $cmd = '') {
    [pscustomobject]@{ ProcessId = $id; ParentProcessId = $parent; Name = $name;
        CreationDate = ([datetime]'2026-11-07T10:00:00Z').AddSeconds($seconds); CommandLine = $cmd }
}
$known = @{}
$snapshot = @(
    (Row 10 1 'chatgpt-overlay-bar.exe' 10),
    (Row 11 10 'msedgewebview2.exe' 11),
    (Row 12 11 'msedgewebview2.exe' 12),
    (Row 80 1 'msedgewebview2.exe' 10),
    (Row 81 80 'msedgewebview2.exe' 11),
    (Row 90 10 'unrelated-reused-parent.exe' 1)
)
$ids = @(Get-GPTrayProcessGroup $snapshot 10 $known | ForEach-Object ProcessId | Sort-Object)
if (($ids -join ',') -ne '10,11,12') { throw "Wrong tree: $ids" }
# A detached surviving WebView is retained; a recycled PID is excluded.
$snapshot = @((Row 12 1 'msedgewebview2.exe' 12), (Row 11 1 'other.exe' 100))
$ids = @(Get-GPTrayProcessGroup $snapshot 10 $known | ForEach-Object ProcessId)
if (($ids -join ',') -ne '12') { throw "PID identity regression: $ids" }
# Recover profile-matched processes even when first snapshot is after reparenting.
$snapshot = @((Row 30 1 'msedgewebview2.exe' 30 '--user-data-dir="C:\Profile With Spaces\EBWebView"'),
    (Row 31 1 'msedgewebview2.exe' 30 '--user-data-dir="C:\Other"'))
$ids = @(Get-GPTrayProcessGroup $snapshot 10 @{} 'C:\Profile With Spaces' | ForEach-Object ProcessId)
if (($ids -join ',') -ne '30') { throw "Profile grouping regression: $ids" }
Write-Output 'Process grouping: 3 scenarios passed (tree, reparent/PID reuse, exact profile).'
# CIM retains microseconds, Get-Process retains FILETIME's final 100ns digit.
$created = [datetime]'2026-11-07T10:00:00.123456Z'
if (!(Test-GPTraySameStartTime $created ($created.AddTicks(9)))) { throw 'CIM precision mismatch rejected a valid process' }
if (Test-GPTraySameStartTime $created ($created.AddTicks(10))) { throw 'Different process start time was accepted' }
# Never accept a replacement root that happens to receive the original PID/name.
$replaced = @((Row 10 1 'chatgpt-overlay-bar.exe' 99))
$group = @(Get-GPTrayProcessGroup $replaced 10 $known)
if ($group.Count -ne 0) { throw 'Recycled root identity accepted' }
if (Test-GPTrayRootPresent $replaced 10 $known) { throw 'Replacement root reported present' }
if (Test-GPTrayRootPresent @() 10 $known) { throw 'Missing root reported present' }
Write-Output 'Timestamp precision and stable root identity: 3 scenarios passed.'
. "$PSScriptRoot/BenchmarkSummary.ps1"
foreach ($case in @(
    @('chatgpt-overlay-bar.exe', '', 'Host'),
    @('msedgewebview2.exe', '--type=renderer', 'Renderer'),
    @('msedgewebview2.exe', '--type=gpu-process', 'GPU'),
    @('msedgewebview2.exe', '--type=utility --utility-sub-type=network.mojom.NetworkService', 'Utility'),
    @('msedgewebview2.exe', '--user-data-dir="C:\Profile"', 'Browser')
)) {
    if ((Get-GPTrayProcessRole $case[0] $case[1]) -ne $case[2]) { throw "Wrong process role: $($case[2])" }
}
$rows = @(
    [pscustomobject]@{VersionLabel='0.2';Scenario='hidden';Complete='True';PrivateWorkingSetMiB=100;PrivateCommitMiB=200;WorkingSetMiB=300;ProcessCount=4},
    [pscustomobject]@{VersionLabel='0.2';Scenario='hidden';Complete='True';PrivateWorkingSetMiB=120;PrivateCommitMiB=240;WorkingSetMiB=360;ProcessCount=6},
    [pscustomobject]@{VersionLabel='0.2';Scenario='hidden';Complete='False';PrivateWorkingSetMiB=999;PrivateCommitMiB=999;WorkingSetMiB=999;ProcessCount=99},
    [pscustomobject]@{VersionLabel='0.3';Scenario='hidden';Complete='True';PrivateWorkingSetMiB=70;PrivateCommitMiB=210;WorkingSetMiB=250;ProcessCount=4}
)
$summary = @(Get-GPTrayMemorySummary $rows)
if ($summary.Count -ne 2) { throw 'Versions were mixed or missing' }
$baseline = @($summary | Where-Object VersionLabel -eq '0.2')[0]
if ($baseline.CompleteSamples -ne 2 -or $baseline.IncompleteSamples -ne 1 -or $baseline.PrivateWorkingSetMeanMiB -ne 110 -or
    $baseline.PrivateCommitMeanMiB -ne 220 -or $baseline.WorkingSetMeanMiB -ne 330 -or $baseline.ProcessCountMean -ne 5) {
    throw 'Wrong complete-only summary or mixed memory metrics'
}
Write-Output 'Process roles and benchmark summary: 6 scenarios passed.'
$wholeQuoted = @((Row 44 1 'msedgewebview2.exe' 44 '"--user-data-dir=C:\Profile With Spaces\EBWebView"'))
if (@(Get-GPTrayProcessGroup $wholeQuoted 10 @{} 'C:\Profile With Spaces').Count -ne 1) { throw 'Whole-quoted profile argument was missed' }
