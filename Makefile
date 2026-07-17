test:
	@ cargo test -- --test-threads=1 --nocapture

run:
	@ cargo run --release -- $(src) debug-output.log

help:
	@ echo "make help: show commands"
	@ echo "make run src=<path>: runs the program with that directory."
	@ echo "\tDebug information stored in debug-output.log"
