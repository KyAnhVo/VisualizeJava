.DEFAULT_GOAL := help

.PHONY: test run help

help:
	@ echo "Make targets"
	@ echo ""
	@ echo "  make help"
	@ echo "    show commands"
	@ echo ""
	@ echo "  make run src=<path>"
	@ echo "    runs the program with that directory."
	@ echo "    Debug information stored in debug-output.log"
	@ echo "    time it take"
	@ echo ""
	@ echo "  make test"
	@ echo "    runs test suite."
	@ echo ""
	@ echo "  make test-with-stdout"
	@ echo "    runs test suite. all println!() are to stdout."

test:
	@ cargo test -- --test-threads=1

test-with-stdout:
	@ cargo test -- --test-threads=1 --nocapture

run:
	@ cargo run --release -- $(src) debug-output.log

