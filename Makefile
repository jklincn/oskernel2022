all:
	cd os && make build
	cp ./os/target/riscv64imac-unknown-none-elf/release/os kernel-qemu
	cp ./bootloader/rustsbi-qemu-0.0.2 sbi-qemu
	
gdb-run:
#	@cd os && make build
	@qemu-system-riscv64 \
		-machine virt \
		-kernel kernel-qemu \
		-m 128M \
		-nographic \
		-smp 2 \
		-bios sbi-qemu \
		-drive file=./simple-fat32/fat32.img,if=none,format=raw,id=x0 \
		-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
		-s -S 

gdb:
	gdb-multiarch \
    -ex 'file ./os/target/riscv64imac-unknown-none-elf/release/os' \
    -ex 'set arch riscv:rv64' \
    -ex 'target remote localhost:1234'

try: all
	@qemu-system-riscv64 \
		-machine virt \
		-kernel kernel-qemu \
		-m 128M \
		-nographic \
		-smp 2 \
		-bios sbi-qemu \
		-drive file=./simple-fat32/fat32.img,if=none,format=raw,id=x0 \
		-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0