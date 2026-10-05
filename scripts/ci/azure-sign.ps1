# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Sign one file with Azure Artifact (Trusted) Signing. Tauri's bundler runs
# this as bundle.windows.signCommand, release.yml only.
#
# Why Tauri has to do the signing for jura-trace.exe: the bundler PATCHES
# the main executable as it bundles (it writes the bundle type, msi or nsis,
# into the binary for the updater), so a signature made before bundling is
# broken by the time the file is inside an installer: HashMismatch, found by
# the release gate on smoke run 37207009853. A signCommand runs after the
# patch, on the bytes that ship.
#
# Only Microsoft's components touch the credentials: signtool and the
# Artifact Signing client library, both installed on the runner by the
# Azure/artifact-signing-action steps that run earlier in the job, and the
# metadata.json that action writes beside the library (endpoint, account,
# certificate profile). The client authenticates through
# DefaultAzureCredential, from AZURE_CLIENT_ID, AZURE_CLIENT_SECRET and
# AZURE_TENANT_ID in the environment.

param([Parameter(Mandatory = $true)][string]$File)
$ErrorActionPreference = "Stop"

$clientRoot = Join-Path $env:LOCALAPPDATA "ArtifactSigning"
$dlib = Get-ChildItem $clientRoot -Recurse -Filter "*Dlib.dll" -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match '\\x64\\' } | Select-Object -First 1
if (-not $dlib) { throw "azure-sign: no Artifact Signing client library under $clientRoot; did an Azure/artifact-signing-action step run earlier in this job?" }
$metadata = Join-Path $dlib.DirectoryName "metadata.json"
if (-not (Test-Path $metadata)) { throw "azure-sign: no metadata.json beside $($dlib.FullName)" }

# signtool: the one the action installed beside the client if there is one,
# else the newest x64 signtool in the Windows SDK on the runner.
$candidates = @(Get-ChildItem $clientRoot, "${env:ProgramFiles(x86)}\Windows Kits\10\bin" -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
                Where-Object { $_.FullName -match '\\x64\\' })
$signtool = $candidates | Where-Object { $_.FullName -like "$clientRoot*" } | Select-Object -First 1
if (-not $signtool) { $signtool = $candidates | Sort-Object FullName -Descending | Select-Object -First 1 }
if (-not $signtool) { throw "azure-sign: signtool.exe not found under $clientRoot or the Windows SDK" }

Write-Host "azure-sign: $File"
& $signtool.FullName sign /fd SHA256 /tr http://timestamp.acs.microsoft.com /td SHA256 `
    /dlib $dlib.FullName /dmdf $metadata $File
if ($LASTEXITCODE -ne 0) { throw "azure-sign: signtool exited $LASTEXITCODE on $File" }

$sig = Get-AuthenticodeSignature $File
if ($sig.Status -ne "Valid") { throw "azure-sign: $File is $($sig.Status) after signing: $($sig.StatusMessage)" }
