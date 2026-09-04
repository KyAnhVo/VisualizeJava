.PHONY: compile_wasm

# Make runs every recipe line in its own shell, so a bare `cd` does not carry
# over to the next line. The build is chained with && for that reason; every
# other path stays relative to the project root.
compile_wasm:
	cd wasm && wasm-pack build --release --target web
	rm -rf demo/src/pkg frontend/src/pkg
	cp -r wasm/pkg demo/src/pkg
	cp -r wasm/pkg frontend/src/pkg

compile_demo:
	cd ./demo && bun run build

compile_frontend:
	cd ./frontend && bun run build
