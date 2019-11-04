
deploy:
  cargo web deploy
  cd target/deploy/; zip -r ludem_dare_45.zip *
  cp target/deploy/ludem_dare_45.zip .

start:
  cargo web start

check:
  cargo check

recheck:
  cargo watch -x check

run:
  RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo run --release

build:
  RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo build --release

debug-build:
  RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo build

rerun:
  RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo watch -x run --release

debug:
  RUST_BACKTRACE=1 cargo run --release
