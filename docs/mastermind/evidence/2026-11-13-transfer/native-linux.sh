set -eu
task_scratch=/mnt/c/Users/joss1/.filesystem-mcp-rs/tmp/vcv-verify-Feynman15-4299
task_vcv=/home/joss/.cache/vcv-verify-Feynman15-4299/debug/vcv-rs
export CUDA_PATH=/usr/local/cuda
export CUDA_HOME=/usr/local/cuda
unset VCPKG_ROOT VCPKG_DEFAULT_TRIPLET NVCC_CCBIN NVCC_PREPEND_FLAGS NVCC_APPEND_FLAGS
"$task_vcv" --cuda on --optix off --no-nvsdk --quiet --format json > "$task_scratch/linux-native-env.json"
"$task_vcv" --cuda on --optix off --no-nvsdk --quiet --format sh > "$task_scratch/linux-native-env.sh"
. "$task_scratch/linux-native-env.sh"
task_bin_dir=$(mktemp -d /tmp/vcv-native-smoke.XXXXXX)
cc "$task_scratch/native.c" -o "$task_bin_dir/native"
task_c_result=$("$task_bin_dir/native")
[ "$task_c_result" = vcv_native_env_ok ]
nvcc "$task_scratch/native.cu" -o "$task_bin_dir/native-cuda"
task_cuda_result=$("$task_bin_dir/native-cuda")
[ "$task_cuda_result" = vcv_cuda_env_ok ]
printf 'BIN_DIR=%s\n' "$task_bin_dir"
printf '%s\n' 'PASS linux generated environment, native C and CUDA host programs'
