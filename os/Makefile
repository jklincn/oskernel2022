# Building
TARGET := riscv64imac-unknown-none-elf
MODE := release
KERNEL_ELF := target/$(TARGET)/$(MODE)/os
KERNEL_BIN := $(KERNEL_ELF).bin

# BOARD
BOOTLOADER := ../bootloader/rustsbi-k210.bin.old
# BOOTLOADER := ../bootloader/rustsbi-k210.bin
K210_BOOTLOADER_SIZE := 131072

BOARD ?= qemu

# Binutils
OBJCOPY := rust-objcopy --binary-architecture=riscv64

build: env initproc_submit $(KERNEL_BIN) 

initproc_submit:
	@cd ../user_c && make build_initproc_submit

env:
	rustup target add $(TARGET)
#	cargo install cargo-binutils --vers =0.3.3
#	rustup component add rust-src
	rustup component add llvm-tools-preview

$(KERNEL_BIN): kernel
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

kernel:
    # @cargo clean
	@echo Platform: $(BOARD)
	@cp src/linker-$(BOARD).ld src/linker.ld
	@cargo build --release --offline --features "board_$(BOARD)"
	@rm src/linker.ld

run: run-inner

copy-to-fs: 
	@cd ../simple-fat32 \
	&& rm -f ./fat32.img \
	&& sh ./create_img.sh \
	&& sudo rm -rf fs \
	&& sudo mkdir fs \
	&& sudo mount fat32.img fs \
	&& sudo cp -r ../test_program/final/stage2/sdcard_content/* fs/ \
	&& sleep 1 \
	&& sudo umount fs \
	&& sudo rmdir fs

run-inner: build
	@qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios ../bootloader/rustsbi-qemu.bin \
		-device loader,file=$(KERNEL_BIN),addr=0x80200000 \
		-drive file=../simple-fat32/fat32.img,if=none,format=raw,id=x0 \
        -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

dump:
	@riscv64-unknown-elf-objdump -S $(KERNEL_ELF) > os_dump

FULLSCORE = 63
test: copy-to-fs 
	@make run > output
ifeq ($(FULLSCORE),$(shell grep -o success output | wc -l ))
	@echo "All test pass!"
else
	@echo "Some test failed, see /os/output"
endif

.PHONY: dump run submit build-custom-test
