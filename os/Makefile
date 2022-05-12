# Building
TARGET := riscv64imac-unknown-none-elf
MODE := release
KERNEL_ELF := target/$(TARGET)/$(MODE)/os
KERNEL_BIN := $(KERNEL_ELF).bin

# BOARD
BOOTLOADER := ../bootloader/rustsbi-k210.bin
K210_BOOTLOADER_SIZE := 131072

BOARD ?= qemu

# Run K210
#K210-SERIALPORT	= /dev/ttyUSB0
#K210-BURNER	= ../tools/kflash.py

# Binutils
OBJCOPY := rust-objcopy --binary-architecture=riscv64

build: env initproc $(KERNEL_BIN) 

initproc:
	@cd ../user_c && make build

env:
	rustup target add $(TARGET)
#	cargo install cargo-binutils --vers =0.3.3
#	rustup component add rust-src
	rustup component add llvm-tools-preview

$(KERNEL_BIN): kernel
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@


kernel:
	@cargo clean
	@echo Platform: $(BOARD)
	@cp src/linker-$(BOARD).ld src/linker.ld
	@cargo build --release --offline --features "board_$(BOARD)"
	@rm src/linker.ld

submit: build
	@cp $(BOOTLOADER) $(BOOTLOADER).copy
	@dd if=$(KERNEL_BIN) of=$(BOOTLOADER).copy bs=$(K210_BOOTLOADER_SIZE) seek=1
	@mv $(BOOTLOADER).copy ../os.bin

run: run-inner

copy_to_fs: 
	cd ../fat32-fuse && sudo mount fat32.img fs && sudo cp ../user_c/bin/* fs/ && sudo umount fs

new_fat32_img:
	cd ../fat32-fuse && rm fat32.img && cp ~/fat32.img .

run-inner: build new_fat32_img
	@qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios ../bootloader/rustsbi-qemu.bin \
		-device loader,file=$(KERNEL_BIN),addr=0x80200000 \
		-drive file=../fat32-fuse/fat32.img,if=none,format=raw,id=x0 \
        -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

.PHONY: build env kernel
