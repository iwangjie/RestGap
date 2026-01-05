Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Push-Location $PSScriptRoot
try {
  if (-not (Get-Command cargo-packager -ErrorAction SilentlyContinue)) {
    cargo install cargo-packager --locked
  }

  if (Get-Command makensis.exe -ErrorAction SilentlyContinue) {
    cargo packager --release --formats nsis
  }
  else {
    Write-Warning "NSIS 未安装：跳过 .exe 打包。建议：choco install nsis -y"
  }

  if (Get-Command candle.exe -ErrorAction SilentlyContinue) {
    cargo packager --release --formats wix
  }
  else {
    Write-Warning "WiX Toolset 未安装：跳过 .msi 打包。建议：choco install wixtoolset -y"
  }
}
finally {
  Pop-Location
}
