set -eu
cd /mnt/c/projects/projects.rust.cg/cglibs/vcv-rs
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=/home/joss/.cache/vcv-verify-Feynman15-4299
task_mask=0
while [ "$task_mask" -lt 16 ]; do
    task_features=
    task_bit=1
    for task_feature in cuda optix nvsdk cli; do
        if [ "$((task_mask & task_bit))" -ne 0 ]; then
            if [ -n "$task_features" ]; then task_features="$task_features,$task_feature"; else task_features="$task_feature"; fi
        fi
        task_bit=$((task_bit * 2))
    done
    set -- check --workspace --all-targets --locked --quiet --no-default-features
    if [ -n "$task_features" ]; then set -- "$@" --features "$task_features"; fi
    printf 'GATE linux feature mask=%s features=%s\n' "$task_mask" "$task_features"
    rustup run 1.98.0 cargo "$@"
    task_mask=$((task_mask + 1))
done
printf '%s\n' 'PASS linux feature matrix 16/16'
