ARCH := riscv64

LIB_DIR := ./lib/*.c
APP_DIR := ./src
TARGET_DIR := ./bin
APPS := $(wildcard $(APP_DIR)/*.c)
# ELFS := $(patsubst $(APP_DIR)/%.c, $(TARGET_DIR)/%, $(APPS))
# BINS := $(patsubst $(APP_DIR)/%.c, $(TARGET_DIR)/%.bin, $(APPS))
ifeq ($(ARCH), riscv64)
LINKER_PATH := ./lib/arch/riscv/user.ld
CC := riscv64-unknown-elf-gcc -ffreestanding -nostdlib -g -mcmodel=medany -I ./include -I ./lib/arch/riscv
endif

bin:
	@rm -rf bin
	@mkdir bin

build: bin
	@$(foreach i_src, $(APPS), $(CC) $(i_src) $(LIB_DIR) -T $(LINKER_PATH) -o $(patsubst $(APP_DIR)/%.c,$(TARGET_DIR)/%,$(i_src));)
