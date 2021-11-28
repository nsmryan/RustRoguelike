
.PHONY: exe run rerun debug release test retest check recheck
exe:
	./target/debug/rl.exe

run:
	cargo run

rerun:
	cargo watch -x run

debug:
	cargo build

release:
	cargo build --release

test:
	cargo test

retest:
	cargo watch -x test

recheck:
	cargo watch -x check

check:
	cargo check

clean:
	cargo clean

