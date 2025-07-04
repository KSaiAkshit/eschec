set unstable

flags := "--release"
DEPTH := "5"
FEN := "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
export RUST_BACKTRACE := "full"

default: play

[doc("Run the engine in play mode")]
play:
    cargo run --bin eschec {{ flags }} -- play

[doc("Run the engine with given args")]
run *args:
    cargo run --bin eschec {{ flags }} -- {{ args }}

[doc("Record perf for given pid")]
record pid:
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
