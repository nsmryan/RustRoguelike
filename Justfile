
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
  RUSTFLAGS="-C link-arg=-fuse-ld=gold" cargo run --release

rerun:
  cargo watch -x run --release

debug:
  RUST_BACKTRACE=1 cargo run --release
