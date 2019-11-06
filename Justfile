
start:
  cargo web start

check:
  cargo check

recheck:
  cargo watch -x check

run:
  RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo run

build:
  RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo build

debug-build:
  RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo build

rerun:
  RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo watch -x run

debug:
  RUST_BACKTRACE=1 cargo run
