user_c: ./user_c/bin
	cd user_c && make build

run: user_c
	cp ./user_c/src/*.c ./user/src/bin 
	cp ./user_c/bin/* ./user/target/riscv64gc-unknown-none-elf/release
	cd os && make run
	rm ./user/src/bin/*.c
