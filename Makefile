clean: test.bc
analyze: sandbox.bc
	@cargo run -- sandbox.bc
sandbox.bc: sandbox.c
	@clang -emit-llvm -o sandbox.bc -c sandbox.c
clean:
	@rm -f sandbox.bc