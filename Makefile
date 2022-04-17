STOP := true

user_c: ./user_c/bin
	cd user_c && make build

kernel: user_c ./os/src ./user/src ./easy-fs/src ./easy-fs-fuse
	cp ./user_c/src/*.c ./user/src/bin 
	cp ./user_c/bin/* ./user/target/riscv64gc-unknown-none-elf/release
	cd os && make build MODE=release
	rm ./user/src/bin/*.c

run: user_c kernel
	qemu-system-riscv64 \
	-machine virt \
	-nographic \
	-bios ./bootloader/rustsbi-qemu.bin \
	-device loader,file=./os/target/riscv64gc-unknown-none-elf/release/os.bin,addr=0x80200000 \
	-drive file=./user/target/riscv64gc-unknown-none-elf/release/fs.img,if=none,format=raw,id=x0 \
	-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
	-s -S

run-imm: 
	qemu-system-riscv64 \
	-machine virt \
	-nographic \
	-bios ./bootloader/rustsbi-qemu.bin \
	-device loader,file=./os/target/riscv64gc-unknown-none-elf/release/os.bin,addr=0x80200000 \
	-drive file=./user/target/riscv64gc-unknown-none-elf/release/fs.img,if=none,format=raw,id=x0 \
	-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
	-s -S

gdb:
	gdb-multiarch \
    -ex 'file ./os/target/riscv64gc-unknown-none-elf/release/os' \
    -ex 'set arch riscv:rv64' \
    -ex 'target remote localhost:1234'