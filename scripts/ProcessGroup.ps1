# Pure grouping logic. Input is a CIM-like snapshot; memory is collected separately.
function Get-GPTrayProcessGroup {
    param([object[]]$Snapshot, [int]$RootProcessId, [hashtable]$Known = @{}, [string]$ProfileDirectory = '')
    $members = @{}
    foreach ($process in $Snapshot) {
        $id = [int]$process.ProcessId
        $identity = '{0}:{1}' -f $id, ([datetime]$process.CreationDate).ToUniversalTime().Ticks
        $profileMatches = $false
        if ($ProfileDirectory -and $process.Name -ieq 'msedgewebview2.exe') {
            $match = [regex]::Match([string]$process.CommandLine, '(?i)(?:^|\s)--user-data-dir=(?:"([^"]+)"|([^\s]+))')
            if ($match.Success) {
                $path = ($match.Groups[1].Value + $match.Groups[2].Value).TrimEnd('\')
                $expected = $ProfileDirectory.TrimEnd('\')
                $profileMatches = $path -ieq $expected -or $path -ieq "$expected\EBWebView"
            }
        }
        # Known identities survive reparenting; creation time prevents PID-reuse contamination.
        $rootMatches = $id -eq $RootProcessId -and $process.Name -ieq 'chatgpt-overlay-bar.exe'
        if ($rootMatches) {
            if (!$Known.ContainsKey('RootIdentity')) { $Known['RootIdentity'] = $identity }
            $rootMatches = $Known['RootIdentity'] -eq $identity
        }
        if ($rootMatches -or $Known.ContainsKey($identity) -or $profileMatches) {
            $members[$id] = $process
        }
    }
    do {
        $added = $false
        foreach ($process in $Snapshot) {
            $id = [int]$process.ProcessId
            $parentId = [int]$process.ParentProcessId
            if (!$members.ContainsKey($id) -and $members.ContainsKey($parentId) -and
                ([datetime]$process.CreationDate) -ge ([datetime]$members[$parentId].CreationDate)) {
                $members[$id] = $process
                $added = $true
            }
        }
    } while ($added)
    foreach ($process in $members.Values) {
        $identity = '{0}:{1}' -f $process.ProcessId, ([datetime]$process.CreationDate).ToUniversalTime().Ticks
        $Known[$identity] = $true
        $process
    }
}
function Test-GPTraySameStartTime {
    param([datetime]$CimTime, [datetime]$ProcessTime)
    $cimTicks = $CimTime.ToUniversalTime().Ticks
    $processTicks = $ProcessTime.ToUniversalTime().Ticks
    return ($cimTicks - ($cimTicks % 10)) -eq ($processTicks - ($processTicks % 10))
}
function Test-GPTrayRootPresent {
    param([object[]]$Snapshot, [int]$RootProcessId, [hashtable]$Known)
    foreach ($process in $Snapshot) {
        $identity = '{0}:{1}' -f $process.ProcessId, ([datetime]$process.CreationDate).ToUniversalTime().Ticks
        if ([int]$process.ProcessId -eq $RootProcessId -and $process.Name -ieq 'chatgpt-overlay-bar.exe' -and
            $Known.ContainsKey('RootIdentity') -and $Known['RootIdentity'] -eq $identity) { return $true }
    }
    return $false
}
