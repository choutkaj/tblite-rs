param([Parameter(Mandatory=$true)][string]$Prefix)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $Prefix).Path
$dlls = @(Get-ChildItem -LiteralPath (Join-Path $root 'bin') -Filter '*tblite*.dll')
if ($dlls.Count -ne 1) { throw 'Expected exactly one tblite DLL in PREFIX/bin' }
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$vs = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vs) { throw 'Install the MSVC C++ Build Tools (x64)' }
$toolDir = Get-ChildItem -LiteralPath (Join-Path $vs 'VC/Tools/MSVC') -Directory | Sort-Object Name -Descending | Select-Object -First 1
$libTool = Join-Path $toolDir.FullName 'bin/Hostx64/x64/lib.exe'
$dumpbin = Join-Path $toolDir.FullName 'bin/Hostx64/x64/dumpbin.exe'
$exports = & $dumpbin /nologo /exports $dlls[0].FullName
if ($LASTEXITCODE -ne 0) { throw 'dumpbin failed' }
$symbols = @($exports | ForEach-Object { if ($_ -match '^\s+\d+\s+[0-9A-F]+\s+[0-9A-F]+\s+(tblite_\w+)\s*$') { $Matches[1] } })
if ($symbols.Count -lt 117) { throw "DLL exports only $($symbols.Count) tblite symbols; expected the complete 0.7 API" }
$expected = (Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot '../tblite-sys/api.json') | ConvertFrom-Json).functions.name
foreach ($symbol in $expected) { if ($symbol -notin $symbols) { throw "Missing public export: $symbol" } }
$def = Join-Path $root 'lib/tblite.def'
@(('LIBRARY ' + $dlls[0].Name), 'EXPORTS') + $symbols | Set-Content -LiteralPath $def -Encoding ascii
& $libTool /nologo /machine:x64 "/def:$def" "/out:$(Join-Path $root 'lib/tblite.lib')"
if ($LASTEXITCODE -ne 0) { throw 'MSVC import library generation failed' }
Write-Output "Created $root/lib/tblite.lib for $($dlls[0].Name)"
