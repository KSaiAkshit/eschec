set unstable

flags := "--release"
export RUST_BACKTRACE := "full"

default: run

run *args:
    cargo run --bin eschec {{ flags }} {{ args }}


[positional-arguments]
perft *args:
    cargo run --bin perft {{ flags }}  $1 "$2"
