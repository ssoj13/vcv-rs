$ErrorActionPreference = 'Stop'
$taskRoot = 'C:\projects\projects.rust.cg\cglibs\vcv-rs'
$taskScratch = 'C:\Users\joss1\.filesystem-mcp-rs\tmp\vcv-verify-Feynman15-4299'
Set-Location -LiteralPath $taskScratch
$taskVcv = Join-Path $taskRoot 'target\debug\vcv-rs.exe'
$env:CUDA_PATH = 'C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.3'
$env:CUDA_HOME = $env:CUDA_PATH
$env:VCPKG_ROOT = $null
$env:VCPKG_DEFAULT_TRIPLET = $null
$env:NVCC_CCBIN = $null
$env:NVCC_PREPEND_FLAGS = $null
$env:NVCC_APPEND_FLAGS = $null
$taskArgs = @('--cuda', 'on', '--optix', 'off', '--no-nvsdk', '--quiet')
$taskJsonText = & $taskVcv @taskArgs --format json
if ($LASTEXITCODE -ne 0) { throw "Environment JSON failed: $LASTEXITCODE" }
$taskJson = ($taskJsonText -join [Environment]::NewLine) | ConvertFrom-Json
if ($taskJson.CUDA_PATH -ne $env:CUDA_PATH) { throw 'CUDA root selection differed from explicit root' }
[IO.File]::WriteAllText((Join-Path $taskScratch 'windows-native-env.json'), ($taskJsonText -join [Environment]::NewLine))
$taskScript = & $taskVcv @taskArgs --format ps
if ($LASTEXITCODE -ne 0) { throw "Environment script failed: $LASTEXITCODE" }
Invoke-Expression ($taskScript -join [Environment]::NewLine)
$taskCExe = Join-Path $taskScratch 'native-windows.exe'
$taskObj = Join-Path $taskScratch 'native-windows.obj'
$taskCSource = Join-Path $taskScratch 'native.c'
& cl.exe /nologo /TC $taskCSource "/Fe$taskCExe" "/Fo$taskObj"
if ($LASTEXITCODE -ne 0) { throw "Native C compilation failed: $LASTEXITCODE" }
$taskCResult = & $taskCExe
if ($LASTEXITCODE -ne 0 -or $taskCResult -ne 'vcv_native_env_ok') { throw 'Native C output mismatch' }
$taskCudaExe = Join-Path $taskScratch 'native-cuda-windows.exe'
$taskCudaSource = Join-Path $taskScratch 'native.cu'
& nvcc.exe $taskCudaSource -o $taskCudaExe
if ($LASTEXITCODE -ne 0) { throw "CUDA host compilation failed: $LASTEXITCODE" }
$taskCudaResult = & $taskCudaExe
if ($LASTEXITCODE -ne 0 -or $taskCudaResult -ne 'vcv_cuda_env_ok') { throw 'CUDA host output mismatch' }
Write-Output 'PASS windows generated environment, native C and CUDA host programs'
