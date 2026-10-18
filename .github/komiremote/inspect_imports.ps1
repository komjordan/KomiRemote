# Evidence only, not a DLL-closure or CRT redistribution approval.
# dumpbin reads PE data; no bundled EXE, DLL or driver is executed.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$vs = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if ($LASTEXITCODE -ne 0 -or -not $vs) { throw 'VS C++ tool installation not found' }
$tools = Join-Path $vs 'VC/Tools/MSVC'
$versions = @(Get-ChildItem $tools -Directory | Sort-Object { [version]$_.Name } -Descending)
if ($versions.Count -eq 0) { throw 'No MSVC tools found' }
$dumpbin = Join-Path $versions[0].FullName 'bin/Hostx64/x64/dumpbin.exe'
if (-not (Test-Path $dumpbin -PathType Leaf)) { throw 'dumpbin x64 missing' }
$output = 'komiremote-test-output/pe-imports'
New-Item $output -ItemType Directory -ErrorAction Stop | Out-Null
$bundle = (Resolve-Path 'rustdesk').Path
$files = @(Get-ChildItem $bundle -File -Recurse | Where-Object { $_.Extension -in '.exe', '.dll', '.sys' } | Sort-Object FullName)
if ($files.Count -eq 0) { throw 'No PE payload found' }
"Reader: $dumpbin`nEvidence only: no dependency closure/architecture/runtime guarantee." | Set-Content "$output/reader.txt" -Encoding utf8
$i = 0
foreach ($file in $files) {
    $relative = [IO.Path]::GetRelativePath($bundle, $file.FullName)
    $report = Join-Path $output ("{0:D4}-{1}.txt" -f $i, $file.Name)
    "Payload: $relative" | Set-Content $report -Encoding utf8
    foreach ($flag in '/HEADERS', '/DEPENDENTS', '/IMPORTS') {
        $lines = & $dumpbin /NOLOGO $flag $file.FullName 2>&1
        if ($LASTEXITCODE -ne 0) { throw "dumpbin failed ($flag): $relative" }
        "`n$flag" | Add-Content $report -Encoding utf8
        $lines | Add-Content $report -Encoding utf8
    }
    $i++
}
Copy-Item 'flutter/build/windows/x64/install_manifest.txt' 'komiremote-test-output/cmake-install-manifest.txt'
