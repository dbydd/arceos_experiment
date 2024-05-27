## 问题

在当前代码中，为usb设备分配地址的代码位于[xhci_usb_device.rs:71](../src/host/structures/xhci_usb_device.rs),

在AddressDevice发出之前，已通过enableSlot命令开启对应槽位，inputContext与outputContext已正确分配
AddressDevice参数经手动读取内存确定正确，inputContext/outputContext参照intel xhci文档分配为64位格式,内存对齐。
发送后返回的TRB completecode为 ParamaterError
请工程师帮忙看看可能是哪里出了问题？

## 前提条件

系统启动流程: uboot启动参数bootcmd='usb start;pci probe'， 而后通过[xmodem](../../../tools/phytium-pi/yet_another_uboot_transfer.py)进行载入，载入到内存位置0x90100000后go 0x90100000启动，启动后进行[完整的xhci控制器初始化流程](../src/host/xhci/mod.rs)（第43行）,而后对每个端口进行[重置+枚举](../src/host/xhci/mod.rs)（入口为第101行）。

初始化流程入口[lib.rs:30](../src/lib.rs)

经日志输出分析可得，程序中所有的内存分配都在0x90100000-0x90200000之间,PageBox所指向的内存区域无cache

官方样例中的中断号不起作用（由于pci没配置），因此使用轮询event ring的方式进行事件处理。

## 怀疑的可能性
pcie上的xhci控制器需要配置pcie的BAR寄存器，但是我们目前拿不到pcie相关的地址，因此我们跳过了pci总线枚举与配置的过程，但是理论上既然uboot已经枚举出了usb设备，那么应当也不存在问题才对。

内存对齐-可能性比较小，可以通过手动读写内存验证都是至少16位对齐了的

## 复现流程：
额外软件：minicom，串口线位于/dev/ttyUSB0，python， pip install pyserial xmodem
uboot启动命令：bootcmd = "usb start; pci probe"
编译启动命令：项目根目录下
```bash
make A=apps/cli PLATFORM=aarch64-phytium-pi ARCH=aarch64 LOG=debug chainboot
```
启动流程：编译启动->脚本等待串口信号->打开飞腾派电源->脚本等待启动命令完成，并刷入编译出来的[bin文件](../../../apps/cli/cli_aarch64-phytium-pi.bin)->脚本将串口转交给minicom，用户通过minicom进行交互->进入系统后测试命令为：test_xhci

系统提供读内存命令 ldr，格式为
```bash
ldr <16进制格式内存地址（无0x前缀，地址8位对齐(即最后一位只能为8/0)> <读取字节数量>
```
（注：读取出的结果为小端序，与intel文档中的布局相同）

## 日志：
```log
arceos# 
arceos# test_xhci
[ 34.218687 0:2 driver_usb::host::xhci:49] resetting xhci controller
[ 34.223450 0:2 driver_usb::host::xhci:50] before reset:UsbStatusRegister { hc_halted: false, host_system_error: false, event_interrupt: false, port_change_detect: true, save_state_status: false, restore_state_status: false, save_restore_error: false, controller_not_ready: false, host_controller_error: false },pagesize: 1
[ 34.253047 0:2 driver_usb::host::xhci:131] stop
[ 34.258777 0:2 driver_usb::host::xhci:136] wait until halt
[ 34.265460 0:2 driver_usb::host::xhci:138] halted
[ 34.271362 0:2 driver_usb::host::xhci:140] HCRST!
[ 34.277266 0:2 driver_usb::host::xhci:166] Reset xHCI Controller Globally
[ 34.285250 0:2 driver_usb::host::xhci:56] pagesize: 1
[ 34.291500 0:2 driver_usb::host::structures::xhci_slot_manager:65] max slot: 16
[ 34.300011 0:2 driver_usb::host::structures::xhci_slot_manager:84] initialized!
[ 34.308516 0:2 driver_usb::host::structures::xhci_slot_manager:21] assign device: VA:0x90192000 to dcbaa 0
[ 34.319364 0:2 driver_usb::host::structures::scratchpad:79] initialized!
[ 34.327263 0:2 driver_usb::host::structures::xhci_roothub:99] number of ports:2
[ 34.335768 0:2 driver_usb::host::structures::xhci_roothub:104] allocating port 0
[ 34.344363 0:2 driver_usb::host::structures:101] ----dumped port state: PortStatusAndControlRegister { current_connect_status: false, port_enabled_disabled: false, over_current_active: false, port_reset: false, port_link_state: 5, port_power: true, port_speed: 0, port_indicator_control: Off, port_link_state_write_strobe: false, connect_status_change: false, port_enabled_disabled_change: false, warm_port_reset_change: false, over_current_change: false, port_reset_change: false, port_link_state_change: false, port_config_error_change: false, cold_attach_status: false, wake_on_connect_enable: false, wake_on_disconnect_enable: false, wake_on_over_current_enable: false, device_removable: false, warm_port_reset: false }
[ 34.408767 0:2 driver_usb::host::structures::xhci_roothub:111] assert:0 == 0
[ 34.417012 0:2 driver_usb::host::structures::xhci_roothub:104] allocating port 1
[ 34.425607 0:2 driver_usb::host::structures:101] ----dumped port state: PortStatusAndControlRegister { current_connect_status: true, port_enabled_disabled: true, over_current_active: false, port_reset: false, port_link_state: 0, port_power: true, port_speed: 4, port_indicator_control: Off, port_link_state_write_strobe: false, connect_status_change: false, port_enabled_disabled_change: false, warm_port_reset_change: false, over_current_change: false, port_reset_change: true, port_link_state_change: false, port_config_error_change: false, cold_attach_status: false, wake_on_connect_enable: false, wake_on_disconnect_enable: false, wake_on_over_current_enable: false, device_removable: false, warm_port_reset: false }
[ 34.489752 0:2 driver_usb::host::structures::xhci_roothub:111] assert:1 == 1
[ 34.497997 0:2 driver_usb::host::structures::xhci_roothub:115] ended
[ 34.505549 0:2 driver_usb::host::structures::xhci_roothub:125] initialized!
[ 34.513745 0:2 driver_usb::host::structures::xhci_command_manager:262] initialized!
[ 34.522561 0:2 driver_usb::host::structures::xhci_event_manager:43] initilizating!
[ 34.531363 0:2 driver_usb::host::structures::event_ring:33] created!
[ 34.538881 0:2 driver_usb::host::structures::xhci_event_manager:56] test
[ 34.546784 0:2 driver_usb::host::structures::xhci_event_manager:99] initialized!
[ 34.555372 0:2 driver_usb::host::xhci:70] before start:UsbStatusRegister { hc_halted: true, host_system_error: false, event_interrupt: false, port_change_detect: true, save_state_status: false, restore_state_status: false, save_restore_error: false, controller_not_ready: false, host_controller_error: false }
[ 34.583843 0:2 driver_usb::host::xhci:81] init completed!, coltroller state:UsbStatusRegister { hc_halted: false, host_system_error: false, event_interrupt: false, port_change_detect: true, save_state_status: false, restore_state_status: false, save_restore_error: false, controller_not_ready: false, host_controller_error: false }
[ 34.614225 0:2 driver_usb::host::xhci:90] port0 : false
[ 34.620647 0:2 driver_usb::host::xhci:90] port1 : true
[ 34.626982 0:2 driver_usb::host::xhci:100] initializing roothub
[ 34.634100 0:2 driver_usb::host::structures::xhci_roothub:45] initializing root ports
[ 34.643127 0:2 driver_usb::host::structures::xhci_roothub:50] initializing port 0
[ 34.651809 0:2 driver_usb::host::structures::root_port:25] port 0 not connected
[ 34.660313 0:2 driver_usb::host::structures::xhci_roothub:50] initializing port 1
[ 34.668994 0:2 driver_usb::host::structures::root_port:28] port 1 connected, continue
[ 34.678020 0:2 driver_usb::host::structures:113] reseting port 1
[ 34.685227 0:2 driver_usb::host::structures:115] before reset Port 1, status: PortStatusAndControlRegister { current_connect_status: true, port_enabled_disabled: true, over_current_active: false, port_reset: false, port_link_state: 0, port_power: true, port_speed: 4, port_indicator_control: Off, port_link_state_write_strobe: false, connect_status_change: false, port_enabled_disabled_change: false, warm_port_reset_change: false, over_current_change: false, port_reset_change: true, port_link_state_change: false, port_config_error_change: false, cold_attach_status: false, wake_on_connect_enable: false, wake_on_disconnect_enable: false, wake_on_over_current_enable: false, device_removable: false, warm_port_reset: false }
[ 34.749896 0:2 driver_usb::host::structures:134] Port 1 reset ok, status: PortStatusAndControlRegister { current_connect_status: true, port_enabled_disabled: true, over_current_active: false, port_reset: false, port_link_state: 0, port_power: true, port_speed: 4, port_indicator_control: Off, port_link_state_write_strobe: false, connect_status_change: false, port_enabled_disabled_change: false, warm_port_reset_change: false, over_current_change: false, port_reset_change: true, port_link_state_change: false, port_config_error_change: false, cold_attach_status: false, wake_on_connect_enable: false, wake_on_disconnect_enable: false, wake_on_over_current_enable: false, device_removable: false, warm_port_reset: false }
[ 34.814211 0:2 driver_usb::host::structures:101] ----dumped port state: PortStatusAndControlRegister { current_connect_status: true, port_enabled_disabled: true, over_current_active: false, port_reset: false, port_link_state: 0, port_power: true, port_speed: 4, port_indicator_control: Off, port_link_state_write_strobe: false, connect_status_change: false, port_enabled_disabled_change: false, warm_port_reset_change: false, over_current_change: false, port_reset_change: true, port_link_state_change: false, port_config_error_change: false, cold_attach_status: false, wake_on_connect_enable: false, wake_on_disconnect_enable: false, wake_on_over_current_enable: false, device_removable: false, warm_port_reset: false }
[ 34.878356 0:2 driver_usb::host::structures::root_port:37] port speed: USBSpeedSuper
[ 34.887295 0:2 driver_usb::host::structures::root_port:39] initializing device: USBSpeedSuper
[ 34.897017 0:2 driver_usb::host::structures::xhci_usb_device:48] new device! port:1
[ 34.905945 0:2 driver_usb::host::structures::root_port:42] writing ...
[ 34.913595 0:2 driver_usb::host::structures::root_port:45] writing complete
[ 34.921754 0:2 driver_usb::host::structures::xhci_usb_device:63] initialize/enum this device! port=1
[ 34.932084 0:2 driver_usb::host::structures::xhci_command_manager:142] do command EnableSlot(EnableSlot { slot_type: 0, cycle_bit: false }) !
[ 34.945973 0:2 driver_usb::host::structures::xhci_command_manager:157] waiting for interrupt handler complete!
[ 34.957171 0:2 driver_usb::host::structures::xhci_event_manager:109] event handler has a trb:PortStatusChange(PortStatusChange { completion_code: Ok(Success), port_id: 2, cycle_bit: true })
[ 34.975223 0:2 driver_usb::host::structures::xhci_event_manager:138] step into port status change.

[ 34.985553 0:2 driver_usb::host::structures::xhci_roothub:78] port 1 changed!
[ 34.993887 0:2 driver_usb::host::structures:101] ----dumped port state: PortStatusAndControlRegister { current_connect_status: true, port_enabled_disabled: true, over_current_active: false, port_reset: false, port_link_state: 0, port_power: true, port_speed: 4, port_indicator_control: Off, port_link_state_write_strobe: false, connect_status_change: false, port_enabled_disabled_change: false, warm_port_reset_change: false, over_current_change: false, port_reset_change: true, port_link_state_change: false, port_config_error_change: false, cold_attach_status: false, wake_on_connect_enable: false, wake_on_disconnect_enable: false, wake_on_over_current_enable: false, device_removable: false, warm_port_reset: false }
[ 35.058032 0:2 driver_usb::host::structures::xhci_event_manager:109] event handler has a trb:PortStatusChange(PortStatusChange { completion_code: Ok(Success), port_id: 2, cycle_bit: true })
[ 35.076085 0:2 driver_usb::host::structures::xhci_event_manager:138] step into port status change.

[ 35.086414 0:2 driver_usb::host::structures::xhci_roothub:78] port 1 changed!
[ 35.094748 0:2 driver_usb::host::structures:101] ----dumped port state: PortStatusAndControlRegister { current_connect_status: true, port_enabled_disabled: true, over_current_active: false, port_reset: false, port_link_state: 0, port_power: true, port_speed: 4, port_indicator_control: Off, port_link_state_write_strobe: false, connect_status_change: false, port_enabled_disabled_change: false, warm_port_reset_change: false, over_current_change: false, port_reset_change: true, port_link_state_change: false, port_config_error_change: false, cold_attach_status: false, wake_on_connect_enable: false, wake_on_disconnect_enable: false, wake_on_over_current_enable: false, device_removable: false, warm_port_reset: false }
[ 35.158893 0:2 driver_usb::host::structures::xhci_event_manager:109] event handler has a trb:CommandCompletion(CommandCompletion { completion_code: Ok(Success), command_trb_pointer: 2417573888, command_completion_parameter: 0, vf_id: 0, slot_id: 1, cycle_bit: true })
[ 35.183717 0:2 driver_usb::host::structures::xhci_command_manager:220] handleing command complete:CommandCompletion { completion_code: Ok(Success), command_trb_pointer: 2417573888, command_completion_parameter: 0, vf_id: 0, slot_id: 1, cycle_bit: true }
[ 35.207328 0:2 driver_usb::host::structures::xhci_command_manager:172] t o t a l success!
[ 35.216701 0:2 driver_usb::host::structures::xhci_usb_device:88] enable slot success! CommandCompletion { completion_code: Ok(Success), command_trb_pointer: 2417573888, command_completion_parameter: 0, vf_id: 0, slot_id: 1, cycle_bit: true }
[ 35.239269 0:2 driver_usb::host::structures::xhci_usb_device:97] init input ctx
[ 35.247777 0:2 driver_usb::host::structures::xhci_usb_device:158] endpoint 0 state: Disabled, slot state: DisabledEnabled
[ 35.259929 0:2 driver_usb::host::structures::xhci_usb_device:132] begin config endpoint 0 and assign dev!
[ 35.270692 0:2 driver_usb::host::structures::xhci_usb_device:135] config ep0
[ 35.278937 0:2 driver_usb::host::structures::xhci_usb_device:158] endpoint 0 state: Disabled, slot state: DisabledEnabled
[ 35.291091 0:2 driver_usb::host::structures::xhci_usb_device:166] assigning device into dcbaa, slot number= 1
[ 35.302199 0:2 driver_usb::host::structures::xhci_slot_manager:21] assign device: VA:0x9019a000 to dcbaa 1
[ 35.313049 0:2 driver_usb::host::structures::xhci_usb_device:178] addressing device
[ 35.321902 0:2 driver_usb::host::structures::xhci_usb_device:180] address to input VA:0xffff000090199000
[ 35.332579 0:2 driver_usb::host::structures::xhci_command_manager:106] addressing device!
[ 35.341953 0:2 driver_usb::host::structures::xhci_command_manager:142] do command AddressDevice(AddressDevice { input_context_pointer: 18446462601150435328, block_set_address_request: false, slot_id: 1, cycle_bit: false }) !
[ 35.363047 0:2 driver_usb::host::structures::xhci_command_manager:157] waiting for interrupt handler complete!
[ 35.374244 0:2 driver_usb::host::structures::xhci_event_manager:109] event handler has a trb:CommandCompletion(CommandCompletion { completion_code: Ok(ParameterError), command_trb_pointer: 2417573904, command_completion_parameter: 0, vf_id: 0, slot_id: 1, cycle_bit: true })
[ 35.399675 0:2 driver_usb::host::structures::xhci_command_manager:220] handleing command complete:CommandCompletion { completion_code: Ok(ParameterError), command_trb_pointer: 2417573904, command_completion_parameter: 0, vf_id: 0, slot_id: 1, cycle_bit: true }
[ 35.423894 0:2 driver_usb::host::structures::xhci_command_manager:176] ok, but: ParameterError
 full trb: CommandCompletion { completion_code: Ok(ParameterError), command_trb_pointer: 2417573904, command_completion_parameter: 0, vf_id: 0, slot_id: 1, cycle_bit: true }
[ 35.448891 0:2 driver_usb::host::structures::xhci_usb_device:191] error while address device at slot id 1
[ 35.459654 0:2 driver_usb::host::structures::xhci_usb_device:199] assert ep0 running!
[ 35.468682 0:2 driver_usb::host::structures::xhci_usb_device:158] endpoint 0 state: Disabled, slot state: DisabledEnabled
[ 35.480834 0:2 driver_usb::host::structures::xhci_usb_device:158] endpoint 0 state: Disabled, slot state: DisabledEnabled
[ 35.492987 0:2 driver_usb::host::structures:101] ----dumped port state: PortStatusAndControlRegister { current_connect_status: true, port_enabled_disabled: true, over_current_active: false, port_reset: false, port_link_state: 0, port_power: true, port_speed: 4, port_indicator_control: Off, port_link_state_write_strobe: false, connect_status_change: false, port_enabled_disabled_change: false, warm_port_reset_change: false, over_current_change: false, port_reset_change: true, port_link_state_change: false, port_config_error_change: false, cold_attach_status: false, wake_on_connect_enable: false, wake_on_disconnect_enable: false, wake_on_over_current_enable: false, device_removable: false, warm_port_reset: false }
[ 35.557130 0:2 driver_usb::host::structures::root_port:49] initialize complete
[ 35.565550 0:2 driver_usb::host::structures::xhci_roothub:54] configuring root ports
arceos# 
```
