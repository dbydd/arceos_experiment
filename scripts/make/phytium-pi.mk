phytium: build
	gzip -9 -cvf $(OUT_BIN) > arceos-phytium-pi.bin.gz
	mkimage -f tools/phytium-pi/phytium-pi.its arceos-phytiym-pi.itb
	@echo 'Built the FIT-uImage arceos-phytium-pi.itb'

chainboot: build
	tools/phytium-pi/yet_another_uboot_transfer.py /dev/ttyUSB0 115200 $(OUT_BIN)
	echo ' ' > minicom_output.log
	minicom -D /dev/ttyUSB0 -b 115200 -C minicom_output.log
# python tools/phytium-pi/uboot_transfer.py /dev/ttyUSB0 115200 $(OUT_BIN)
#	python tools/phytium-pi/uboot_test_send.py /dev/ttyUSB0 115200 $(OUT_BIN)
#ruby tools/phytium-pi/uboot_transfer.rb /dev/ttyUSB0 115200 $(OUT_BIN)
	
