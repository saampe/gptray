# Diagnostic only: identify unavailable Windows exports before native main starts.
param([string]$Executable = (Join-Path $PSScriptRoot '../src-tauri/target/x86_64-pc-windows-msvc/debug/examples/native_smoke.exe'))
$ErrorActionPreference = 'Stop'
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$visualStudio = & $vswhere -latest -products '*' -property installationPath
$dumpbin = Get-ChildItem "$visualStudio/VC/Tools/MSVC/*/bin/Hostx64/x64/dumpbin.exe" | Sort-Object FullName -Descending | Select-Object -First 1
if (!$dumpbin) { throw 'MSVC dumpbin not found' }
$imports = & $dumpbin.FullName /imports $Executable
$imports | Set-Content 'native-imports.txt' -Encoding utf8
$current = ''
$handle = [IntPtr]::Zero
foreach ($line in $imports) {
    if ($line -match '^\s+([A-Za-z0-9_.-]+\.dll)\s*$') {
        if ($handle -ne [IntPtr]::Zero) { [System.Runtime.InteropServices.NativeLibrary]::Free($handle) }
        $current = $Matches[1]
        $handle = [IntPtr]::Zero
        if (![System.Runtime.InteropServices.NativeLibrary]::TryLoad($current, [ref]$handle)) { Write-Output "MISSING_MODULE: $current" }
    } elseif ($handle -ne [IntPtr]::Zero -and $line -match '^\s+[0-9A-Fa-f]{1,5}\s+([A-Za-z_?@][A-Za-z0-9_?@$]+)\s*$') {
        $name = $Matches[1]
        $address = [IntPtr]::Zero
        if (![System.Runtime.InteropServices.NativeLibrary]::TryGetExport($handle, $name, [ref]$address)) { Write-Output "MISSING_IMPORT: $current!$name" }
    }
}
if ($handle -ne [IntPtr]::Zero) { [System.Runtime.InteropServices.NativeLibrary]::Free($handle) }
Write-Output 'Native import inspection complete.'
