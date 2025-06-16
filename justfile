set unstable

flags := "--release"
DEPTH := "5"
FEN := "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
export RUST_BACKTRACE := "full"

default: play

play:
    cargo run --bin eschec {{ flags }} -- play

run *args:
    cargo run --bin eschec {{ flags }} {{ args }}

record pid:
    perf record --call-graph dwarf -p {{ pid }}

[positional-arguments]
perft depth=DEPTH fen=FEN:
    cargo run --bin eschec {{ flags }} -- perft -d {{ depth }} --fen "{{ fen }}"
