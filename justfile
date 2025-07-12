set unstable := true

flags := "--release"
DEPTH := "5"
FEN := "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
pgn_output_dir := 'guantlet/results/'
book_file := 'guantlet/books/2moves.pgn'
export RUST_BACKTRACE := "full"

alias up := update
alias log := tail_log
alias tail := tail_log
alias b := build
alias r := run

default: play

[doc("Print help")]
help:
    just -l

[doc("Build and symlink binary")]
update:
    -rm ./guantlet/engines/eschec
    just build
    ln -s ../../target/release/eschec guantlet/engines/

[doc("Run a gauntlet match against another engine using cutechess-cli")]
[positional-arguments]
gauntlet opponent='gnuchess' rounds='40' tc='15+0.1' concurrency='4' : update
    @# Print the configuration for this run
    @echo "Starting gauntlet:"
    @echo "  - Opponent: {{ opponent }}"
    @echo "  - Rounds: {{ rounds }}"
    @echo "  - Time Control: {{ tc }}"
    @echo "  - PGN Output: {{ pgn_output_dir }}eschec_vs_{{ opponent }}"
    @echo "-------------------------------------"

    @# Run the cutechess-cli command
    cutechess-cli \
        -engine conf=lucia \
        -engine conf={{ opponent }} \
        -each tc={{ tc }} \
        -rounds {{ rounds }} \
        -openings file={{ book_file }} format=pgn order=random \
        -pgnout {{ pgn_output_dir }}eschec_vs_{{ opponent }}.txt \
        -concurrency {{ concurrency }} \
        -draw movenumber=40 movecount=8 score=20 \
        -resign movecount=3 score=800 \
        -recover

[doc("Remove build artifacts and logs")]
clean:
    rm -rf ./logs ./guantlet/results/* ./guantlet/engines/*

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
