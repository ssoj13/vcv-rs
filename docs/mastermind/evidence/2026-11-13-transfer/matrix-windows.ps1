$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath 'C:\projects\projects.rust.cg\cglibs\vcv-rs'
$env:CARGO_BUILD_JOBS = '2'
$env:CARGO_TARGET_DIR = 'C:\projects\projects.rust.cg\cglibs\vcv-rs\target'
$taskFeatures = @('cuda', 'optix', 'nvsdk', 'cli')
for ($taskMask = 0; $taskMask -lt 16; $taskMask++) {
    $taskSelected = @()
    for ($taskBit = 0; $taskBit -lt 4; $taskBit++) {
        if (($taskMask -band (1 -shl $taskBit)) -ne 0) { $taskSelected += $taskFeatures[$taskBit] }
    }
    $taskCargoArgs = @('+1.98.0', 'check', '--workspace', '--all-targets', '--locked', '--quiet', '--no-default-features')
    if ($taskSelected.Count -gt 0) { $taskCargoArgs += @('--features', ($taskSelected -join ',')) }
    Write-Output ("GATE windows feature mask={0} features={1}" -f $taskMask, ($taskSelected -join ','))
    & cargo @taskCargoArgs
    if ($LASTEXITCODE -ne 0) { throw "Feature mask $taskMask failed with exit $LASTEXITCODE" }
}
Write-Output 'PASS windows feature matrix 16/16'
