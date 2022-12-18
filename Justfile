name := 'spectrusty-tutorial'
features := env_var_or_default('SPECTRUSTY_FEATURES', "")
llvm_profdata_exe := replace(clean(`rustc --print target-libdir` / ".." / "bin" / "llvm-profdata"),'\','/')
target := replace_regex(trim_end_match(`rustup default`, ' (default)'), '^[^-]+-', '')
optimizations := '-Zno-parallel-llvm -Ccodegen-units=1'
default_tap := 'resources/iana128.tap'

# build all examples
steps:
    cargo build --bins --release

# run step5
run args=default_tap:
    cargo run --bin step5 --features="{{features}}" --release -- {{args}}

# run step5 with CPU frequency reporting
run-measure args=default_tap:
    cargo run --bin step5 --features="measure_cpu_freq,{{features}}" --release -- {{args}}

# run step5 MIR optimized (rustc nightly)
run-mir args=default_tap: rustcwrap
    RUSTFLAGS="{{optimizations}}" RUSTC_WRAPPER="./rustcwrap" \
        cargo +nightly-{{target}} run --target="{{target}}" --bin step5 --features="{{features}}" --release \
            -- {{args}}

# run step5 profile generate (rustc nightly)
run-profgen args=default_tap:
    RUSTFLAGS="-Cprofile-generate=tmp/pgo-data" cargo +nightly-{{target}} run --target="{{target}}" --bin step5 --features="{{features}}" --release -- {{args}}

# run step5 with profiled optimizations (rustc nightly)
run-prof args=default_tap:
    set -euxo pipefail
    # rustup component add llvm-tools-preview
    {{llvm_profdata_exe}} merge -o tmp/pgo-data/merged.profdata tmp/pgo-data
    RUSTFLAGS="-Cllvm-args=-pgo-warn-missing-function -Cprofile-use={{justfile_directory()}}/tmp/pgo-data/merged.profdata" \
        cargo +nightly-{{target}} run --target="{{target}}" --bin step5 --features="{{features}}" --release \
            -- {{args}}

# build rustcwrap for MIR builds
rustcwrap:
    rustc rustcwrap.rs -o rustcwrap.exe

# clean cargo and profile data
clean:
    cargo clean
    rm -rf tmp/pgo-data
