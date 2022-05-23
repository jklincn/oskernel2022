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