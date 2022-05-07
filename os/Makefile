# Building
TARGET := riscv64imac-unknown-none-elf
MODE := release
KERNEL_ELF := target/$(TARGET)/$(MODE)/os
KERNEL_BIN := $(KERNEL_ELF).bin

# BOARD
BOOTLOADER := ../bootloader/rustsbi-k210.bin
K210_BOOTLOADER_SIZE := 131072

# Run K210
#K210-SERIALPORT	= /dev/ttyUSB0
#K210-BURNER	= ../tools/kflash.py

# Binutils
OBJCOPY := rust-objcopy --binary-architecture=riscv64

build: env $(KERNEL_BIN) initproc

initproc:
	@cd ../user && make build

env:
	rustup target add $(TARGET)
#	cargo install cargo-binutils --vers =0.3.3
#	rustup component add rust-src
	rustup component add llvm-tools-preview

$(KERNEL_BIN): kernel
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

kernel:
	@echo Platform: $(BOARD)
	@cp src/linker-$(BOARD).ld src/linker.ld
	@cargo build --release --offline --features "board_$(BOARD)"
	@rm src/linker.ld

submit: build
	@cp $(BOOTLOADER) $(BOOTLOADER).copy
	@dd if=$(KERNEL_BIN) of=$(BOOTLOADER).copy bs=$(K210_BOOTLOADER_SIZE) seek=1
	@mv $(BOOTLOADER).copy ../os.bin


.PHONY: build env kernel
