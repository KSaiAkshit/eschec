set unstable

flags := "--release"
export RUST_BACKTRACE := "full"

default: run

run:
    cargo run --bin eschec {{ flags }}


[positional-arguments]
perft *args:
    cargo run --bin perft {{ flags }}  $1 "$2"
