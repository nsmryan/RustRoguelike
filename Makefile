ATLAS = "tools/atlas"
ATLAS_SRC = $(ATLAS)/main.c $(ATLAS)/util.c $(ATLAS)/bitmap.c $(ATLAS)/lib/stb/stb_image.c $(ATLAS)/lib/stb/stb_image_write.c $(ATLAS)/lib/stb/stb_rect_pack.c $(ATLAS)/lib/stb/stb_truetype.c

CC ?= gcc

all: run

.PHONY: exe run rerun debug release test retest check recheck sloc sloc_crates atlas clean unsave

run: resources/spriteAtlas.png resources/spriteAtlas.txt
	cargo run

resources/spriteAtlas.png: atlas

resources/spriteAtlas.txt: atlas

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

sloc:
	cloc */src/*.rs --by-file

sloc_crates:
	cloc roguelike_utils/src/*.rs
	cloc roguelike_core/src/*.rs
	cloc roguelike_lib/src/*.rs
	cloc roguelike_display/src/*.rs
	cloc roguelike_draw/src/*.rs
	cloc roguelike_engine/src/*.rs
	cloc roguelike_main/src/*.rs
	cloc roguelike_map/src/*.rs

atlas:
	@echo "building atlas executable"
	@$(CC) -std=gnu99 -O0 -o atlas $(ATLAS_SRC) -lm
	@echo "collecting images"
	@rm collectImages -rf
	@mkdir collectImages
	@find resources/animations -name "*.png" | xargs -I{} cp {} collectImages/
	@find resources/UI -name "*.png" | xargs -I{} cp {} collectImages/
	@find resources/misc -name "*.png" | xargs -I{} cp {} collectImages/
	@find resources/tileset -name "*.png" | xargs -I{} cp {} collectImages/
	#@cp resources/rustrogueliketiles.png collectImages/
	@echo "building atlas image"
	@./atlas collectImages/ --imageout resources/spriteAtlas.png --textout resources/spriteAtlas.txt
	@rm collectImages -rf
	@echo "done"

unsave:
	-@rm game.save

clean:
	@rm atlas
	@cargo clean
	@rm collectImages -rf
	@rm game.save

