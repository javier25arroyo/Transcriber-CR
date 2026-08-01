#requires -Version 5
<#
.SYNOPSIS
    Corre un comando de cargo con el entorno que necesita la feature `whisper`.

.DESCRIPTION
    `whisper-rs` compila whisper.cpp con CMake y genera sus bindings con bindgen
    (libclang). En Windows, en una máquina que no sea de desarrollo de C++, eso
    pide tres cosas que cargo por sí solo no encuentra:

      * libclang.dll (LLVM)  -> variable LIBCLANG_PATH
      * cmake.exe            -> en el PATH
      * los headers del CRT de MSVC y del Windows SDK, más los builtin de clang
        -> variable BINDGEN_EXTRA_CLANG_ARGS, porque el driver de clang que usa
           bindgen NO lee la variable INCLUDE del entorno de desarrollo de VS.

    Este script descubre esas rutas (no las hardcodea: usa vswhere, el LLVM
    instalado y la última versión del SDK) y después ejecuta `cargo` con los
    argumentos que se le pasen.

.EXAMPLE
    ./scripts/build-whisper.ps1 build -p stt-sidecar --features whisper --release

.EXAMPLE
    ./scripts/build-whisper.ps1 test -p stt-sidecar --features whisper -- --ignored
#>

# Sin param()/CmdletBinding a propósito: así PowerShell no intenta interpretar
# banderas como `-p` como parámetros de este script y las deja en $args para
# pasárselas tal cual a cargo. Todo lo que va después del nombre del script es
# la línea de cargo a ejecutar.
$CargoArgs = $args

$ErrorActionPreference = 'Stop'

function Find-Dir([string]$description, [string[]]$candidates) {
    foreach ($c in $candidates) {
        if ($c -and (Test-Path $c)) { return (Resolve-Path $c).Path }
    }
    throw "No se encontró $description. Revisá que esté instalado (ver CONTRIBUTING.md § Setup)."
}

# --- libclang (LLVM) ---------------------------------------------------------
$llvmRoot = Find-Dir 'LLVM (libclang)' @(
    "$env:ProgramFiles\LLVM",
    "${env:ProgramFiles(x86)}\LLVM"
)
$libclangBin = Join-Path $llvmRoot 'bin'
# Los headers builtin de clang (stdbool.h, stddef.h, ...): lib\clang\<ver>\include
$clangInclude = Get-ChildItem (Join-Path $llvmRoot 'lib\clang') -Directory |
    Sort-Object Name -Descending | Select-Object -First 1 |
    ForEach-Object { Join-Path $_.FullName 'include' }
if (-not $clangInclude) { throw "No se encontraron los headers builtin de clang bajo $llvmRoot\lib\clang." }

# --- CMake -------------------------------------------------------------------
$cmakeBin = $null
if (Get-Command cmake -ErrorAction SilentlyContinue) {
    $cmakeBin = Split-Path (Get-Command cmake).Source
} else {
    $cmakeBin = Join-Path (Find-Dir 'CMake' @("$env:ProgramFiles\CMake")) 'bin'
}

# --- MSVC (headers del compilador) -------------------------------------------
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (-not (Test-Path $vswhere)) { throw "No se encontró vswhere; instalá Visual Studio 2022 con el toolset de C++." }
$vsRoot = & $vswhere -latest -property installationPath
$msvcInclude = Get-ChildItem (Join-Path $vsRoot 'VC\Tools\MSVC') -Directory |
    Sort-Object Name -Descending | Select-Object -First 1 |
    ForEach-Object { Join-Path $_.FullName 'include' }
if (-not $msvcInclude) { throw "No se encontraron los headers de MSVC bajo $vsRoot." }

# --- Windows SDK (ucrt / shared / um) ----------------------------------------
$sdkRoot = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\Include" -Directory |
    Sort-Object Name -Descending | Select-Object -First 1 | ForEach-Object { $_.FullName }
if (-not $sdkRoot) { throw "No se encontró el Windows SDK bajo Windows Kits\10\Include." }

# --- Armar el entorno y correr cargo -----------------------------------------
$env:LIBCLANG_PATH = $libclangBin
$env:PATH = "$cmakeBin;$env:PATH"

$includes = @($msvcInclude, "$sdkRoot\ucrt", "$sdkRoot\shared", "$sdkRoot\um", $clangInclude)
$env:BINDGEN_EXTRA_CLANG_ARGS = ($includes | ForEach-Object { "-I`"$_`"" }) -join ' '

Write-Host 'Entorno para la feature whisper:'
Write-Host "  LIBCLANG_PATH = $env:LIBCLANG_PATH"
Write-Host "  cmake         = $cmakeBin"
Write-Host "  includes      = $($includes.Count) rutas (-I) para bindgen"
Write-Host ''

if (-not $CargoArgs -or $CargoArgs.Count -eq 0) {
    Write-Host 'Sin argumentos de cargo; sólo se preparó el entorno. Ejemplo:'
    Write-Host '  ./scripts/build-whisper.ps1 build -p stt-sidecar --features whisper'
    return
}

Write-Host "cargo $($CargoArgs -join ' ')"
& cargo @CargoArgs
exit $LASTEXITCODE
