set unstable := true

flags := "--release"
DEPTH := "5"
FEN := "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
pgn_output_dir := 'gauntlet/results/'
engines_dir := "./gauntlet/engines"
book_dir := 'gauntlet/books/'
engine_logs_dir := '/tmp/eschec_logs/'
test_suite_dir := './test_suites/sts'
test_suite_blitz_file := './test_suites/sts_blitz.epd'
epd_test_depth := "8"
repo_url := env('REPO_URL')
perf_reps := "5"
perf_stat_events := "cycles,instructions,branches,branch-misses,cache-references,cache-misses"
OUT_DIR := "/tmp/out_dir"
BIN_NAME := "eschec"
params_file := "./config/normalized_tuned_params.toml"
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

build-all-tags:
    #!/usr/bin/bash
    set -euo pipefail

    TMP_DIR="$(mktemp -d)"
    echo "Cloning repo into $TMP_DIR"
    git clone --quiet "{{ repo_url }}" "$TMP_DIR"

    pushd "$TMP_DIR" > /dev/null

    echo "Fetching tags..."
    git fetch --tags --quiet
    TAGS=$(git tag)

    for tag in $TAGS; do
        echo "Building for tag: $tag"
        git checkout --quiet "$tag"

        just build

        if [ -f "{{ BIN_NAME }}" ]; then
            cp "{{ BIN_NAME }}" "{{ OUT_DIR }}/{{ BIN_NAME }}-${tag}"
        elif [ -f "target/release/{{ BIN_NAME }}" ]; then
            cp "target/release/{{ BIN_NAME }}" "{{ OUT_DIR }}/{{ BIN_NAME }}-${tag}"
        else
            echo "Warning: Could not find built binary for $tag"
        fi
    done

    popd > /dev/null
    rm -rf "$TMP_DIR"
    echo "All binaries saved to {{ OUT_DIR }}"

[doc("Build and symlink binary")]
update bin="eschec":
    -rm {{ engines_dir }}/eschec_old
    -mv {{ engines_dir }}/eschec {{ engines_dir }}/eschec_old
    just build {{ bin }}
    cp target/release/eschec {{ engines_dir }}

[doc("Warmup with the given binary and then run perf stat on it")]
stat bin: setup
    @echo "Warming up 3x"
    @for i in $(seq 1 3); do {{ bin }}; done
    @echo "{{ MAGENTA }}Starting perf stat{{ NORMAL }}"
    perf stat -r {{ perf_reps }} -e {{ perf_stat_events }} {{ bin }}
    notify-send "Done stating"

spsa sts='./test_suites/sts/' threads='0' time_ms='1000' iter='200':
    @echo "{{ MAGENTA }} Starting SPSA Tuning {{ NORMAL }}"
    cargo run --bin tuner --features parallel --release -- {{ sts }} --threads {{ threads }} \
        --time-ms {{ time_ms }} --iterations {{ iter }} \
        | tee {{ pgn_output_dir }}spsa_tuning_log_{{ threads }}t_{{ time_ms }}ms_{{ iter }}.txt

[doc("Run a gauntlet match against another engine using cutechess-cli")]
[positional-arguments]
gauntlet opponent='gnuchess' rounds='40' concurrency='4' book='8moves_v3.pgn' tc='30+0.3': update
    @# Print the configuration for this run
    @echo "Starting gauntlet:"
    @echo "  - Opponent: {{ BLUE }}{{ opponent }}{{ NORMAL }}"
    @echo "  - Rounds: {{ GREEN }}{{ rounds }}{{ NORMAL }}"
    @echo "  - Book: {{ MAGENTA }}{{ book }}{{ NORMAL }}"
    @echo "  - Time Control: {{ CYAN }}{{ tc }}{{ NORMAL }}"
    @echo "  - PGN Output: {{ YELLOW }}{{ pgn_output_dir }}eschec_vs_{{ opponent }}{{ NORMAL }}"
    @echo "-------------------------------------"

    # -draw movenumber=40 movecount=8 score=20 \
    # -resign movecount=3 score=800 \
    @# Run the cutechess-cli command
    cutechess-cli \
        -engine conf=eschec \
        -engine conf={{ opponent }} \
        -each tc={{ tc }} \
        -rounds {{ rounds }} \
        -openings file={{ book_dir }}{{ book }} format={{ extension(book_dir + book) }} order=random policy=round \
        -pgnout {{ pgn_output_dir }}eschec_vs_{{ opponent }}.txt \
        -concurrency {{ concurrency }} \
        -recover

[doc("Run an SPRT test between two versions of the engine.")]
[positional-arguments]
sprt p1 p2 rounds='100' concurrency='4' book='8moves_v3.pgn' tc='30+0.3':
    @# Print the configuration for this run
    @echo "Starting gauntlet:"
    @echo "  - Engine1: {{ BLUE }}{{ p1 }}{{ NORMAL }}"
    @echo "  - Engine2: {{ BLUE }}{{ p2 }}{{ NORMAL }}"
    @echo "  - Rounds: {{ GREEN }}{{ rounds }}x2{{ NORMAL }}"
    @echo "  - Book: {{ MAGENTA }}{{ book }}{{ NORMAL }}"
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
        -openings file={{ book_dir }}{{ book }} format={{ extension(book_dir + book) }} order=random \
        -sprt elo0=0 elo1=10 alpha=0.05 beta=0.05 \
        -repeat -recover \
        -pgnout {{ pgn_output_dir }}{{ p1 }}_vs_{{ p2 }}_sprt.txt \
        -log file={{ engine_logs_dir }}sprt_log.txt level=info \
        | tee {{ pgn_output_dir }}{{ p1 }}_vs_{{ p2 }}_sprt_log.txt

[doc("Run a full validation test at a fixed depth")]
[group("eval")]
eval_validation_test:
    cargo run --release --features parallel --bin eval_tester {{ test_suite_dir }} -p {{ params_file }} -d {{ epd_test_depth }} --threads 0 \
    | tee {{ pgn_output_dir }}{{ datetime("%Y-%m-%d_%H-%M-%S") }}_validation_test_log.txt

[doc("Run a fast 'smoke_test' on a small set of positions")]
[group("eval")]
eval_blitz_test:
    cargo run {{ flags }} --features parallel --bin eval_tester {{ test_suite_blitz_file }} -p {{ params_file }} -d {{ epd_test_depth }} --threads 0  \
    | tee {{ pgn_output_dir }}{{ datetime("%Y-%m-%d_%H-%M-%S") }}_blitz_test.txt

[doc("Run eval_tester sequentially with given time control.")]
[group("eval")]
eval_test_seq tc='stc' suite=test_suite_dir:
    cargo run {{ flags }} --bin eval_tester {{ suite }} -p {{ params_file }} -t {{ tc }} \
    | tee {{ pgn_output_dir }}{{ datetime("%Y-%m-%d_%H-%M-%S") }}_test_{{ tc }}_sequential.txt

[doc("Run eval_tester with given time control and threads")]
[group("eval")]
eval_test threads="6" tc='stc' suite=test_suite_dir:
    cargo run {{ flags }} --features parallel --bin eval_tester {{ suite }} -p {{ params_file }} -t {{ tc }} --threads {{ threads }} \
    | tee {{ pgn_output_dir }}{{ datetime("%Y-%m-%d_%H-%M-%S") }}_test_{{ tc }}_{{ threads }}threads.txt

[doc("Remove build artifacts and gauntlet artifacts")]
clean:
    rm -rf ./gauntlet/results/
    mkdir ./gauntlet/results

[doc("Run the engine in play mode")]
play:
    cargo run --bin eschec {{ flags }} -- --params ./config/normalized_tuned_params.toml play

[doc("Run the engine in headless mode")]
uci:
    cargo run --bin eschec {{ flags }} -- headless --protocol uci

[doc("Run the engine with given args")]
run *args:
    cargo run --bin eschec {{ flags }} -- {{ args }}

[doc("Find the most recent file in the 'logs' directory and tail it")]
@tail_log:
    echo "{{ MAGENTA }} Tailing log {{ NORMAL }}"
    tail -f {{ engine_logs_dir }}$(ls -t {{ engine_logs_dir }} | head -n 1) | bat --paging=never --language log --plain

[doc("Record perf with given args")]
record args="": setup
    just build
    perf record --call-graph dwarf ./target/release/eschec {{ args }}

[doc("Run with given args along with dhat feature")]
dhat-heap args="":
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
@build bin="eschec":
    cargo build --bin {{ bin }} {{ flags }}

[doc("Set some variables for debugging")]
@setup:
    echo "-1" | sudo tee /proc/sys/kernel/perf_event_paranoid
    echo "0" | sudo tee /proc/sys/kernel/kptr_restrict
    echo "0" | sudo tee /proc/sys/kernel/nmi_watchdog

[doc("Generate flamegraph for given args and view in flamelens")]
flame *args: setup
    cargo flamegraph --post-process 'flamelens --echo' {{ flags }} --bin eschec --  --params ./config/normalized_tuned_params.toml {{ args }}

[doc("lint the codebase")]
lint:
    cargo clippy --all-targets --all-features -- -D warnings

[doc("check the codebase")]
check:
    cargo check --all-targets --all-features

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
