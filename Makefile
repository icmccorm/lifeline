analyze: sandbox.bc
	@cargo run -- sandbox.bc
sandbox.bc: sandbox.c
	@clang -emit-llvm -o sandbox.bc -c sandbox.c

clean:
	@rm -f sandbox.bc
	@rm -rf ./target

docs:
	cargo doc --no-deps --lib
	rm -rf ./docs
	echo "<meta http-equiv=\"refresh\" content=\"0; url=build_wheel\">" > target/doc/index.html
	cp -r target/doc ./docs