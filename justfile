set unstable := true

flags := "--release"
DEPTH := "5"
FEN := "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
pgn_output_dir := 'gauntlet/results/'
book_file := 'gauntlet/books/2moves.pgn'
engine_logs_dir := '/tmp/eschec_logs/'
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
    -rm ./gauntlet/engines/eschec
    just build
    cp target/release/eschec gauntlet/engines/

[doc("Run a gauntlet match against another engine using cutechess-cli")]
[positional-arguments]
gauntlet opponent='gnuchess' rounds='40' concurrency='4' tc='15+0.1': update
    @# Print the configuration for this run
    @echo "Starting gauntlet:"
    @echo "  - Opponent: {{ BLUE }}{{ opponent }}{{ NORMAL }}"
    @echo "  - Rounds: {{ GREEN }}{{ rounds }}{{ NORMAL }}"
    @echo "  - Time Control: {{ CYAN }}{{ tc }}{{ NORMAL }}"
    @echo "  - PGN Output: {{ YELLOW }}{{ pgn_output_dir }}eschec_vs_{{ opponent }}{{ NORMAL }}"
    @echo "-------------------------------------"

    @# Run the cutechess-cli command
    cutechess-cli \
        -engine conf=eschec \
        -engine conf={{ opponent }} \
        -each tc={{ tc }} \
        -rounds {{ rounds }} \
        -openings file={{ book_file }} format=pgn order=random \
        -pgnout {{ pgn_output_dir }}eschec_vs_{{ opponent }}.txt \
        -concurrency {{ concurrency }} \
        -draw movenumber=40 movecount=8 score=20 \
        -resign movecount=3 score=800 \
        -recover

[doc("Run an SPRT test between two versions of the engine.")]
[positional-arguments]
sprt p1 p2 rounds='100' concurrency='4' tc='15+0.1':
    @# Print the configuration for this run
    @echo "Starting gauntlet:"
    @echo "  - Engine1: {{ BLUE }}{{ p1 }}{{ NORMAL }}"
    @echo "  - Engine2: {{ BLUE }}{{ p2 }}{{ NORMAL }}"
    @echo "  - Rounds: {{ GREEN }}{{ rounds }}x2{{ NORMAL }}"
    @echo "  - Time Control: {{ CYAN }}{{ tc }}{{ NORMAL }}"
    @echo "  - Output: {{ YELLOW }}{{ pgn_output_dir }}{{ p1 }}_vs_{{ p2 }}_sprt.txt{{ NORMAL }}"
    @echo "-------------------------------------"

    @echo "{{ GREEN }}Starting SPRT test...{{ NORMAL }}"
    @fastchess \
        -engine cmd=./gauntlet/engines/{{ p1 }} name={{ p1 }} \
        -engine cmd=./gauntlet/engines/{{ p2 }} name={{ p2 }} \
        -each tc={{ tc }} \
        -rounds {{ rounds }} \
        -concurrency {{ concurrency }} \
        -openings file={{ book_file }} format=pgn order=random \
        -sprt elo0=0 elo1=10 alpha=0.05 beta=0.05 \
        -repeat -recover \
        -log file={{ engine_logs_dir }}sprt_log.txt \
        | tee {{ pgn_output_dir }}{{ p1 }}_vs_{{ p2 }}_sprt.txt

[doc("Remove build artifacts and gauntlet artifacts")]
clean:
    rm -rf ./gauntlet/results/ ./gauntlet/engines/
    mkdir ./gauntlet/results ./gauntlet/engines

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
@tail_log:
    echo "{{ MAGENTA }} Tailing log {{ NORMAL }}"
    tail -f {{ engine_logs_dir }}$(ls -t {{ engine_logs_dir }} | head -n 1)

[doc("Record perf with given args")]
record args="": setup
    just build
    perf record --call-graph dwarf ./target/release/eschec {{ args }}

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
    cargo nextest run

[doc("Build in release mode")]
@build:
    cargo build --bin eschec {{ flags }}

[doc("Set some variables for debugging")]
@setup:
    echo "-1" | sudo tee /proc/sys/kernel/perf_event_paranoid
    echo "0" | sudo tee /proc/sys/kernel/kptr_restrict
    echo "0" | sudo tee /proc/sys/kernel/nmi_watchdog

[doc("Generate flamegraph for given args and view in flamelens")]
flame *args: setup
    cargo flamegraph --post-process 'flamelens --echo' {{ flags }} --bin eschec -- {{ args }}

[doc("lint the codebase")]
lint:
    cargo clippy --all-targets --all-features -- -D warnings

[doc("check format")]
fmt-check:
    -cargo fmt --all --check
    just fmt

[confirm("Run 'cargo fmt'?")]
fmt:
    cargo fmt

[doc("run perft at given depth and fen")]
[positional-arguments]
perft depth=DEPTH fen=FEN:
    cargo run --bin eschec {{ flags }} -- perft -d {{ depth }} --fen "{{ fen }}"

[doc("run perft divide at given depth and fen")]
[positional-arguments]
perftd depth=DEPTH fen=FEN:
    cargo run --bin eschec {{ flags }} -- perft -d {{ depth }} --fen "{{ fen }}" --divide
