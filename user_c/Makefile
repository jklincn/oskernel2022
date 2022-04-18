CC := riscv64-unknown-elf-gcc -ffreestanding -nostdlib -g -mcmodel=medany -I ./include
LIB_DIR := ./lib/*.c
APP_DIR := ./src
TARGET_DIR := ./bin
APPS := $(wildcard $(APP_DIR)/*.c)
# ELFS := $(patsubst $(APP_DIR)/%.c, $(TARGET_DIR)/%, $(APPS))
# BINS := $(patsubst $(APP_DIR)/%.c, $(TARGET_DIR)/%.bin, $(APPS))

bin:
	mkdir bin

build: bin
	$(foreach i_src, $(APPS), $(CC) $(i_src) $(LIB_DIR) -T ./linker.ld -o $(patsubst $(APP_DIR)/%.c,$(TARGET_DIR)/%,$(i_src));)
