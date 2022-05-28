all:
	cd os && make submit BOARD=k210


# BOARD
BOOTLOADER := bootloader/rustsbi-k210.bin
#BOOTLOADER := /home/oslab/Desktop/jeremy_code/rCore-Tutorial-v3/bootloader/rustsbi-k210.bin
K210_BOOTLOADER_SIZE := 131072
K210-SERIALPORT	= /dev/ttyUSB0
K210-BURNER	= ./tools/kflash.py

KERNEL_BIN := os.bin
#KERNEL_BIN := /home/oslab/Desktop/jeremy_code/rCore-Tutorial-v3/os/target/riscv64gc-unknown-none-elf/release/os.bin

run: all
#	(which $(K210-BURNER)) || (cd .. && git clone https://github.com/sipeed/kflash.py.git && mv kflash.py tools)
# @cp $(BOOTLOADER) $(BOOTLOADER).copy
# @dd if=$(KERNEL_BIN) of=$(BOOTLOADER).copy bs=$(K210_BOOTLOADER_SIZE)
# @mv $(BOOTLOADER).copy $(KERNEL_BIN)
	@sudo chmod 777 $(K210-SERIALPORT)
	python3 $(K210-BURNER) -p $(K210-SERIALPORT) -b 1500000 $(KERNEL_BIN)
	python3 -m serial.tools.miniterm --eol LF --dtr 0 --rts 0 --filter direct $(K210-SERIALPORT) 115200
	
gdb-run:
#	@cd os && make build
	@qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios ./bootloader/rustsbi-qemu.bin \
		-device loader,file=os/target/riscv64imac-unknown-none-elf/release/os.bin,addr=0x80200000 \
		-drive file=./fat32-fuse/fat32.img,if=none,format=raw,id=x0 \
        -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
		-s -S 

gdb:
	gdb-multiarch \
    -ex 'file ./os/target/riscv64imac-unknown-none-elf/release/os' \
    -ex 'set arch riscv:rv64' \
    -ex 'target remote localhost:1234'