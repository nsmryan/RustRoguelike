
.PHONY: exe run rerun debug release test retest check recheck sloc
run:
	cargo run

exe:
	./target/debug/rl.exe

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

sloc:
	cloc */src/*.rs --by-file

