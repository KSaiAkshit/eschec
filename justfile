set unstable := true

flags := "--release"
DEPTH := "5"
FEN := "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
export RUST_BACKTRACE := "full"

alias up := update
alias log := tail_log
alias tail := tail_log

default: play

[doc("Build and symlink binary")]
update:
    -rm ./eschec
    just build
    ln -s ./target/release/eschec .

[doc("Remove build artifacts and logs")]
clean:
    cargo clean
    rm -rf ./logs ./eschec

[doc("Run the engine in play mode")]
play:
    cargo run --bin eschec {{ flags }} -- play

[doc("Run the engine in headless mode")]
uci:
    cargo run --bin eschec {{ flags }} -- headless --protocol uci

[doc("Run the engine with given args")]
run *args:
    cargo run --bin eschec {{ flags }} -- {{ args }}

[doc("Find the most recent file in the 'logs' directory and tail it")]
tail_log:
    tail -f ./logs/$(ls -t logs | head -n 1)

[doc("Record perf for given pid")]
record pid: setup
    perf record --call-graph dwarf -p {{ pid }}

[doc("Run with given args along with dhat feature")]
dhat-heap args="":
    RUSTFLAGS="-Cprofile-use" \
    DHAT_PROFILE=1 \
    cargo run --profile release-dhat --features dhat-heap --bin eschec -- {{ args }}

[doc("Run perft along with dhat feature")]
dhat-perft depth=DEPTH fen=FEN:
    DHAT_PROFILE=1 \
    cargo run --profile release-dhat --features dhat-heap --bin eschec -- perft -d {{ depth }} --fen "{{ fen }}"

[doc("Run all tests")]
test:
    cargo test --all-features

[doc("Build in release mode")]
@build:
    cargo build --bin eschec {{ flags }}

[doc("Set some variables for debugging")]
@setup:
    echo "-1" | sudo tee /proc/sys/kernel/perf_event_paranoid
    echo "0" | sudo tee /proc/sys/kernel/kptr_restrict
    echo "0" | sudo tee /proc/sys/kernel/nmi_watchdog

[doc("Generate flamegraph for perft and view in flamelens")]
flame depth: setup
    cargo flamegraph --post-process 'flamelens --echo' {{ flags }} --bin eschec -- perft --depth {{ depth }}

[doc("lint the codebase")]
lint:
    cargo clippy --all-targets --all-features -- -D warnings

[doc("check format")]
fmt-check:
    cargo fmt --all --check

[doc("run perft at given depth and fen")]
[positional-arguments]
perft depth=DEPTH fen=FEN:
    cargo run --bin eschec {{ flags }} -- perft -d {{ depth }} --fen "{{ fen }}"

[doc("run perft divide at given depth and fen")]
[positional-arguments]
perftd depth=DEPTH fen=FEN:
    cargo run --bin eschec {{ flags }} -- perft -d {{ depth }} --fen "{{ fen }}" --divide
