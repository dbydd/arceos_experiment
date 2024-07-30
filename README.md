# proj2210132-基于飞腾派的Arceos移植与外设驱动开发-设计文档

* 队名：旋转轮椅
* 成员：姚宏伟，郭天宇，蒋秋吉
* 学校：上海建桥学院

[TOC]

## 如何运行复现？这里是使用手册

使用手册遵循Arceos开发的惯例，其位于[doc目录](./doc/)下

## 目标概述：
飞腾派开发板是飞腾公司针对教育行业推出的一款国产开源硬件平台，兼容 ARMv8-A 处理器架构。飞腾小车是一款利用飞腾派开发板组装的小车，它是一个计算机编程实验平台，包含电机驱动、红外传感器、超声传感器、AI加速棒、视觉摄像云台等，可以满足学生学习计算机编程、人工智能训练等课题。

ArceOS是基于组件化设计的思路，用Rust语言的丰富语言特征，设计实现不同功能的独立操作系统内核模块和操作系统框架，可形成不同特征/形态/架构的操作系统内核。

![arceos](/Users/dbydd/Documents/oscpmp/doc/figures/ArceOS.svg)

本项目需要参赛队伍**将ArceOS移植到飞腾派开发板**，实现**UART串口通信**、**以太网口通信（HTTP server和HTTP client 可以上传下载文件）**，并且可以通过实现**I2C驱动**和**USB驱动**去驱动小车行走。
* 完成ArceOS操作系统移植到飞腾派开发板。
* 实现UART串口通信。
* 实现以太网口通信（HTTP server和HTTP client 可以上传下载文件）。
* 通过实现I2C驱动PCA9685模块来驱动电机实现小车行走。
* 通过实现USB驱动利用遥控手柄实现小车行走。

## 完成概况：
1. 系统移植：完成

2. UART串口通信：完成

3. 以太网口通信：尚未完成

4. i2c驱动：已跑通，尚未驱动小车上的驱动板

5. USB驱动：情况较为复杂，详见进展情况说明
   5.1.  XHCI主机驱动：完成
   	5.1.1. 通用性usb主机端驱动框架重构：完成
   5.2.  HID：完成，已有鼠标测试用例
   5.3. UVC（摄像头）：未完成，但是编写了一套可行的驱动框架

## 在一切开始前，前人所作的工作&开发进展情况说明

本仓库基于[该分支](https://github.com/limingth/arceos)进行开发
* 飞腾派所使用的UART控制器为[PL011](https://developer.arm.com/documentation/ddi0183/latest/)，是一款被广泛应用的适用于Arm平台的芯片，因此，该芯片的驱动有较多实例可供参考。
* 以太网口驱动：
* i2c驱动：无，逆向官方的sdk，从0开始开发。
* USB驱动：本任务中所指的USB驱动，是指USB在主机端的一整套子系统，就从软件层面来说，其包含[XHCI(USB3.0主机控制器)](./doc/resources/[XHCI控制器规范]extensible-host-controler-interface-usb-xhci.pdf)驱动与[USB协议层驱动](./doc/resources/[USB3.2协议规范]USB%203.2%20Revision%201.1.pdf)
    * XHCI驱动：分为两部分，首先是主机本身的控制程序，然后是基于XHCI的USB设备枚举程序
        * 主机控制程序: 参见XHCI规范第四章(Operation Model),这部分主要负责主机寄存器的相关操作，与硬件关联的数据结构的管理(如控制器在内存中的抽象，控制器所使用的内存的管理(TRB环，中断向量等))
        * 设备枚举程序: 在主机控制程序完成后，我们还需要根据USB规范，对所连接的设备进行一次扫描，扫描过程中需要对设备做对应的初始化（这部分是通用操作，包含启用USB端点，选择设备配置等）
    * USB协议层：USB规范是类似于OSI网络七层架构一样的设计，换句话来说，USB规范并不是单本规范，而是**一套**规范，在USB所定义的四种传输类型（传输层）的基础上，则是随着USB设备类型的日益增加而变的[无比庞大](https://www.usbzh.com/article/special.html)的协议层，举几个例子：
        * [HID(Human Interface Device)协议](./doc/resources/HID/hid1_11.pdf)指的是人类交互设备，如：鼠标，键盘，手柄等设备。
        * [UVC(USB video Class)协议](./doc/resources/[UVC]/UVC%201.5%20Class%20specification.pdf)是是微软与另外几家设备厂商联合推出的为USB视频捕获设备定义的协议标准
        * ...
    * 在更之上，还有诸多厂家自定义的协议，因此，在我们从0开始，创新性的使用RUST编写操作系统的情况下，想在这么短的时间内，凭借不多的人手去移植整个USB系统是几乎不可能的，因此我们选择，**从协议层开始，先仅仅移植出一套可以形成控制闭环的最小系统，并且在编写的过程中做好整个USB子系统的设计，以方便日后的来者进行开发**
    * 从另一方面来说，用Rust所编写的完整的USB主机驱动也很少，因此可以拿来参考的案例也比较少，Redox算是一个

## 工作日志

### 初赛

#### 第一阶段-系统移植&串口移植-2024.3-2024.4

| 时间节点  | 里程碑                                                       |
| --------- | ------------------------------------------------------------ |
| 2024/3/27 | 收到设备，初步配置开发环境                                   |
| 2024/3/29 | dump出了内存布局，新建并修改了arceos的平台描述文档使其适配平台的内存结构 |
| 2024/3/30 | 开始串口调试，进入Uboot，确认路线：通过uboot(tftp加载)引导arceos镜像，并实验成功 |
| 2024/4/3  | 系统移植成功，能够通过板子上的gpio进行串口通信，可以与系统的命令行交互 |

#### 第二阶段-USB系统移植-跑通前

| 时间节点  | 里程碑-USB系统                                               | 里程碑-i2c驱动          |
| --------- | ------------------------------------------------------------ | ----------------------- |
| 2024/4/7  | 收到飞腾小车，开始研究小车的结构，同时开始抽时间研究i2c驱动  |                         |
| 2024/4/9  | 开始编写usb系统的驱动                                        |                         |
| 2024/5/1  | Xhci控制器初始化完成                                         |                         |
| 2024/5/15 | 成功通过控制器给设备分配了地址（即-第一条xhci控制命令的成功发送） |                         |
| 2024/5/17 |                                                              | 编译出了官方sdk中的demo |
| 2024/6/15 | 开启了设备的控制端点，能够进行控制传输，并编写了初赛所需要的文档 |                         |

### 决赛
#### 第二阶段-USB系统移植-跑通
| 时间节点  | 里程碑-USB系统                                               | 里程碑-i2c驱动                   |
| --------- | ------------------------------------------------------------ | -------------------------------- |
| 2024/6/20 | 获取到了设备的描述符，并简单编写了相应的描述符解析代码       |                                  |
| 2024/7/10 | 成功根据设备的端点配置好了设备的其余通信断电，决定先写个hid驱动 |                                  |
| 2024/7/15 | 鼠标驱动demo大体完成，能够获取单次回报数据，开始检修bug      | 开始根据sdk编写i2c驱动以驱动小车 |
| 2024/7/18 | 经排查，定位到bug问题出现在传输环的翻转位上，经过修复后可以正常建立有线鼠标的通信 |                                  |
| 2024/7/20 | 成功编写出无线鼠标的驱动（即-实现了复合hid设备）             |                                  |

#### 第三阶段-提供跨平台/跨操作系统移植友好性的usb驱动框架-重构&uvc摄像头驱动开发

| 时间节点  | 里程碑-USB系统                                               | 里程碑-i2c驱动                                             |
| --------- | ------------------------------------------------------------ | ------------------------------------------------------------ |
| 2024/7/22 | 经研究与实际需求相结合，决定开始编写uvc摄像头驱动 |  |
| 2024/7/23 | 已经到了不得不重构的时候了，开始重构整个usb框架 |  |
| 2024/7/29 | 完成了框架的重构，将原有的hid驱动搬到了新的框架下，原有代码位于[phytium_pi_dev]([Files · phytium_pi_dev · 旋转轮椅 / proj2210132-基于飞腾派的Arceos移植与外设驱动开发 · GitLab (eduxiji.net)](https://gitlab.eduxiji.net/T202412799992620/project2210132-232991/-/tree/phytium_pi_dev?ref_type=heads))分支 | 代码编写完成，开始debug |
| ...至今 | 仍然在进行uvc摄像头的开发，进展良好 |  |


## 关于系统移植的工作
——多说无益，展示代码！

### 系统移植
由于Arceos从设计开始就以跨平台为目标，因此已经有了较为完善的启动[配置文件](./platforms/aarch64-phytium-pi.toml):
```toml
#./platforms/aarch64-phytium-pi.toml
# Architecture identifier.
arch = "aarch64"
# Platform identifier.
platform = "aarch64-phytium-pi"
# Platform family.
family = "aarch64-phytium-pi"

# Base address of the whole physical memory.
phys-memory-base = "0x8000_0000"
# Size of the whole physical memory.
phys-memory-size = "0x8000_0000" # 2G
# Base physical address of the kernel image.
kernel-base-paddr = "0x9010_0000"
# Base virtual address of the kernel image.
kernel-base-vaddr = "0xffff_0000_9010_0000"
# kernel-base-vaddr = "0x9010_0000"
# Linear mapping offset, for quick conversions between physical and virtual
# addresses.
 phys-virt-offset = "0xffff_0000_0000_0000"
#phys-virt-offset = "0x0000_0000_0000_0000"
# MMIO regions with format (`base_paddr`, `size`).
mmio-regions = [
  # ["0xFE00_B000", "0x1000"],      # mailbox
  # ["0xFE20_1000", "0x1000"],      # PL011 UART
  # ["0xFF84_1000", "0x8000"],      # GICv2    
  #["0x40000000", "0xfff_ffff"],      # pcie ecam


  # ["0x6_0000_0000", "0x4000_0000"] # pcie control


  ["0x2800_C000", "0x1000"],      # UART 0
  ["0x2800_D000", "0x1000"],      # UART 1
  ["0x2800_E000", "0x1000"],      # UART 2
  ["0x2800_F000", "0x1000"],      # UART 3
  # ["0x32a0_0000", "0x2_0000"],      # usb0
  # ["0x32a2_0000", "0x2_0000"],      # usb0
  # ["0x3200_C000", "0x2000"],      #Ethernet1
  # ["0x3200_E000", "0x2000"],      #Ethernet2
  # ["0x3080_0000", "0x8000"],      # GICv2    
  ["0x3000_0000","0x800_0000"],     #other devices
  ["0x4000_0000","0x4000_0000"],   # pcie 
  ["0x10_0000_0000", "0x20_0000_0000"], # pcie mmio 64

  # ["0x6_0000_0000", "0x6_3fff_ffff"] # pcie control
]
virtio-mmio-regions = []
# UART Address
uart-paddr = "0x2800_D000"
uart-irq = "24"

# GIC Address
gicc-paddr = "0xFF84_2000"
gicd-paddr = "0xFF84_1000"

# Base physical address of the PCIe ECAM space.
pci-ecam-base = "0x4000_0000"
# End PCI bus number.
pci-bus-end = "0x100"
# PCI device memory ranges.
pci-ranges = [
  ["0x58000000", "0x27ffffff"],   # pcie mmio 32
  ["0x10_0000_0000", "0x30_0000_0000"], # pcie mmio 64
  # ["0x5800_0000", "0x7fff_ffff"],

  # ["0x6_0000_0000", "0x6_3fff_ffff"]
]

# Size of the nocache memory region
nocache-memory-size = "0x70_0000"
```

此外还需要修改一些Makefile
```makefile
#Makefile
#...
ifeq ($(PLATFORM_NAME), aarch64-raspi4)
  include scripts/make/raspi4.mk
else ifeq ($(PLATFORM_NAME), aarch64-phytium-pi)
  include scripts/make/phytium-pi.mk #添加编译平台目标
else ifeq ($(PLATFORM_NAME), aarch64-bsta1000b)
  include scripts/make/bsta1000b-fada.mk
endif
#...
```

为了开发时的方便起见，我们还写了给自动上传编译结果至uboot以供引导的小脚本
```makefile
#scripts/make/phytium-pi.mk
#添加启动项
phytium: build #build出目标镜像并打包成uboot可用格式
	gzip -9 -cvf $(OUT_BIN) > arceos-phytium-pi.bin.gz
	mkimage -f tools/phytium-pi/phytium-pi.its arceos-phytiym-pi.itb
	@echo 'Built the FIT-uImage arceos-phytium-pi.itb'

chainboot: build #无需手动载入镜像，使用此启动项可以将编译结果自动加载进uboot
	tools/phytium-pi/yet_another_uboot_transfer.py /dev/ttyUSB0 115200 $(OUT_BIN)
	echo ' ' > minicom_output.log
	minicom -D /dev/ttyUSB0 -b 115200 -C minicom_output.log
```

```python
#tools/phytium-pi/yet_another_uboot_transfer.py
#!/usr/bin/env python3

#串口传输脚本

import sys
import time
import serial
from xmodem import XMODEM

def send_file(port, baudrate, file_path):
    # 打开串口
    ser = serial.Serial(port, baudrate, timeout=1)
    
    # 等待 U-Boot 提示符
    while True:
        line = ser.readline().decode('utf-8', errors='ignore').strip()
        print(line)
        if line.endswith('Phytium-Pi#'):
            break
    
    # 发送 loady 命令
    ser.write(b'loadx 0x90100000\n')
    time.sleep(0.5)
    
    # 等待 U-Boot 准备好接收文件
    while True:
        line = ser.readline().decode('utf-8', errors='ignore').strip()
        print(line)
        if 'Ready for binary' in line:
            break
    
    # 发送 'C' 字符开始传输
    ser.write(b'C')
    
    # 使用 xmodem 协议传输文件
    with open(file_path, 'rb') as f:
        def getc(size, timeout=1):
            return ser.read(size) or None

        def putc(data, timeout=1):
            return ser.write(data)

        modem = XMODEM(getc, putc)
        modem.send(f)
    
    # 关闭串口
    ser.close()

if __name__ == '__main__':
    if len(sys.argv) != 4:
        print("Usage: python script.py <port> <baudrate> <file_path>")
        sys.exit(1)
    
    port = sys.argv[1]
    baudrate = int(sys.argv[2])
    file_path = sys.argv[3]
    
    send_file(port, baudrate, file_path)
```

### 串口适配
代码详见该[crate文件夹](./crates/arm_pl011/src/pl011.rs)，其启动步骤大致为：

根据[手册](./doc/resources/飞腾派软件编程手册V1.0.pdf)定义需要的寄存器（默认波特率是115200，无需定义处理波特率相关寄存器）

```rust
register_structs! {
    /// Pl011 registers.
    Pl011UartRegs {
        /// Data Register.
        (0x00 => dr: ReadWrite<u32>),
        (0x04 => _reserved0),
        /// Flag Register.
        (0x18 => fr: ReadOnly<u32>),
        (0x1c => _reserved1),
        /// Control register.
        (0x30 => cr: ReadWrite<u32>),
        /// Interrupt FIFO Level Select Register.
        (0x34 => ifls: ReadWrite<u32>),
        /// Interrupt Mask Set Clear Register.
        (0x38 => imsc: ReadWrite<u32>),
        /// Raw Interrupt Status Register.
        (0x3c => ris: ReadOnly<u32>),
        /// Masked Interrupt Status Register.
        (0x40 => mis: ReadOnly<u32>),
        /// Interrupt Clear Register.
        (0x44 => icr: WriteOnly<u32>),
        (0x48 => @END),
    }
}
```

实现初始化，读写字符，响应中断

```rust
/// The Pl011 Uart
///
/// The Pl011 Uart provides a programing interface for:
/// 1. Construct a new Pl011 UART instance
/// 2. Initialize the Pl011 UART
/// 3. Read a char from the UART
/// 4. Write a char to the UART
/// 5. Handle a UART IRQ
pub struct Pl011Uart {
    base: NonNull<Pl011UartRegs>,
}

unsafe impl Send for Pl011Uart {}
unsafe impl Sync for Pl011Uart {}

impl Pl011Uart {
    /// Constrcut a new Pl011 UART instance from the base address.
    pub const fn new(base: *mut u8) -> Self {
        Self {
            base: NonNull::new(base).unwrap().cast(),
        }
    }

    const fn regs(&self) -> &Pl011UartRegs {
        unsafe { self.base.as_ref() }
    }

    /// Initializes the Pl011 UART.
    ///
    /// It clears all irqs, sets fifo trigger level, enables rx interrupt, enables receives
    pub fn init(&mut self) {
        // clear all irqs
        self.regs().icr.set(0x7ff);

        // set fifo trigger level
        self.regs().ifls.set(0); // 1/8 rxfifo, 1/8 txfifo.

        // enable rx interrupt
        self.regs().imsc.set(1 << 4); // rxim

        // enable receive
        self.regs().cr.set((1 << 0) | (1 << 8) | (1 << 9)); // tx enable, rx enable, uart enable
    }

    /// Output a char c to data register
    pub fn putchar(&mut self, c: u8) {
        while self.regs().fr.get() & (1 << 5) != 0 {}
        self.regs().dr.set(c as u32);
    }

    /// Return a byte if pl011 has received, or it will return `None`.
    pub fn getchar(&mut self) -> Option<u8> {
        if self.regs().fr.get() & (1 << 4) == 0 {
            Some(self.regs().dr.get() as u8)
        } else {
            None
        }
    }

    /// Return true if pl011 has received an interrupt
    pub fn is_receive_interrupt(&self) -> bool {
        let pending = self.regs().mis.get();
        pending & (1 << 4) != 0
    }

    /// Clear all interrupts
    pub fn ack_interrupts(&mut self) {
        self.regs().icr.set(0x7ff);
    }
}
```

### I2C移植
TODO

### USB驱动
由于USB驱动要解释起来较为复杂，因此我们新开一章来说明我们的工作。


# USB驱动代码导读

## 目的&前言

我们的最终目的是形成一个跨平台的、操作系统无关的、no_std环境下的usb host(xhci)库，本章节的目的在于通过引领读者阅读一遍已完成的小目标的代码来让读者熟悉目前的项目进展与框架。

在本章节中，我们将会结合目前已有的代码来学习如何从0开始编写一个主机端的USB鼠标驱动。

## 系统架构

我们所期望的，在逻辑上的系统结构如下:

![系统结构图](./doc/figures/main_architecture.png)

## 开始

我们的主机端驱动基于XHCI控制器，因此我们先从XHCI开始.

### 控制器

首先是控制器的启用与初始化，遵循[xhci文档](https://chenlongos.com/Phytium-Car/assert/extensible-host-controler-interface-usb-xhci.pdf)第四章(Operation Model)的描述，我们进行如下操作

* [代码地址（见调用此方法处）](https://gitlab.eduxiji.net/T202412799992620/project2210132-232991/blob/phytium_pi_dev/crates/driver_usb/src/host/xhci/mod.rs#L80)

```rust
//...
impl<O> Xhci<O>
where
    O: OsDep,
{
    fn init(&mut self) -> Result {
        self.chip_hardware_reset()?;    //重置控制器
        self.set_max_device_slots()?;   //配置设备槽位
        self.set_dcbaap()?;             //设置device context数组基地址
        self.set_cmd_ring()?;           //配置命令环
        self.init_ir()?;                //配置中断寄存器组

        self.setup_scratchpads();       //配置xhci控制器所使用的内存
        self.start()?;                  //启动控制器

        self.test_cmd()?;               //验证命令环
        self.reset_ports();             //重置端口
        Ok(())
    }
//...
```

### 枚举设备

在控制器初始化完成后，就开始枚举USB设备并分配驱动，目前我们这一块的代码比较简陋

* [开始枚举](https://gitlab.eduxiji.net/T202412799992620/project2210132-232991/blob/phytium_pi_dev/apps/usb/src/main.rs#L49)

* [poll函数的调用处](https://gitlab.eduxiji.net/T202412799992620/project2210132-232991/blob/phytium_pi_dev/crates/driver_usb/src/host/mod.rs#L109)

* [poll函数的实现](https://gitlab.eduxiji.net/T202412799992620/project2210132-232991/blob/phytium_pi_dev/crates/driver_usb/src/host/xhci/mod.rs#L127)

```rust
//...
    fn poll( //此函数同样的，遵循xhci文档第四章的部分
        &mut self,
        arc: Arc<SpinNoIrq<Box<dyn Controller<O>>>>,
    ) -> Result<Vec<DeviceAttached<O>>> {
        let mut port_id_list = Vec::new();
        let port_len = self.regs().port_register_set.len();
        for i in 0..port_len { //确保每一个port都已经被正确的初始化，并且将连接上设备的port记录下来(由于多线程在飞腾派上不完善，我们目前没有热拔插功能)
            let portsc = &self.regs_mut().port_register_set.read_volatile_at(i).portsc;
            info!(
                "{TAG} Port {}: Enabled: {}, Connected: {}, Speed {}, Power {}",
                i,
                portsc.port_enabled_disabled(),
                portsc.current_connect_status(),
                portsc.port_speed(),
                portsc.port_power()
            );

            if !portsc.port_enabled_disabled() {
                continue;
            }

            port_id_list.push(i);
        }
        let mut device_list = Vec::new();
        for port_idx in port_id_list { //为每一个设备初始化
            let port_id = port_idx + 1;
            let slot = self.device_slot_assignment()?;  //向控制器请求分配一个slot号，作为设备的标识符
            let mut device = self.dev_ctx.new_slot(     //将设备与slot绑定
                slot as usize,
                0,
                port_id,
                32,
                self.config.os.clone(),
                arc.clone(),
            )?;
            debug!("assign complete!");
            self.address_device(&device)?;              //向控制器通告设备与slot之间的绑定关系

            self.print_context(&device);

//-----------------------------------------从这里开始进入USB的部分-----------------------------------------//
            let packet_size0 = self.fetch_package_size0(&device)?; //获取*端点0*的包传输大小 --之后会讲

            debug!("packet_size0: {}", packet_size0);

            self.set_ep0_packet_size(&device, packet_size0); //正确配置端点0的传输包大小
            let desc = self.fetch_device_desc(&device)?;//获取设备的描述符
            let vid = desc.vendor; //生产厂家id
            let pid = desc.product_id;//产品id

            // debug!("current state:");
            // self.debug_dump_output_ctx(slot.into());

            info!("device found, pid: {pid:#X}, vid: {vid:#X}");

            device.device_desc = desc;

            trace!(
                "fetching device configurations, num:{}",
                device.device_desc.num_configurations
            );
            for i in 0..device.device_desc.num_configurations {//对于一个usb设备，其内置了厂家定义的数个可选的工作模式/配置(config),在这里，我们获取所有的配置
                let config = self.fetch_config_desc(&device, i)?;//发送控制传输请求：获取配置描述符
                trace!("{:#?}", config);
                device.configs.push(config)
            }

            //TODO: set interface 1?

            // device.set_current_interface(1); //just change this line to switch interface //interface：当设备是一个复合设备的时候，就通过interface来定义不同的功能与其对应的端点。在代码测试所用的无线鼠标上，不需要手动设置interface，因为其默认interface就是鼠标，但是视情况不同，可能需要手动配置

            self.set_configuration(&device, 0)?; //进入设备的初始化阶段

            device_list.push(device);
        }
        Ok(device_list)
    }
//...
```

在以上代码中，我们提到了以下概念：

* 端点：[又称为通道](https://www.usbzh.com/article/detail-445.html)，是传输数据的抽象载体，端点一共有3+1种类型：
    1. 中断端点(Interrupt Endpoint)：此类端点对应于[中断传输](https://www.usbzh.com/article/detail-109.html)用于传输数据量不大，但是要求实时性的数据，如HID设备(Human Interface Device，人类交互设备，如鼠标/键盘/手柄等)的回报报文。
    2. 同步端点(Isochronous)：此类端点对应于[同步传输](https://www.usbzh.com/article/detail-118.html)用于数据连续、实时且大量的数据传输,如：摄像头。
    3. 块端点(Bulk Endpoint)：此类端点对应于[块传输](https://www.usbzh.com/article/detail-40.html)，用于数据量大但对实时性要求不高的场合，也就是传文件。
    4. 控制端点(Control Endpoint/Endpoint 0)：在[USB协议文档](https://chenlongos.com/Phytium-Car/assert/[USB3.2协议规范]USB%203.2%20Revision%201.1.pdf)中，明确的定义了0号端点为控制端点，这个端点永远处于可用状态，其对应的[控制传输](https://www.usbzh.com/article/detail-55.html)负责控制/获取USB设备的状态
  * USB设备初始状态下只有一个0号端点-即控制端点是固定的，其余的端点都处于未配置状态-此时端点的类型是没有配置的
  * 对于具体的哪个端点是什么类型，这些信息都被存储在[Interface描述符](https://www.usbzh.com/article/detail-64.html)下属的[端点描述符](https://www.usbzh.com/article/detail-56.html)中
    * Q:什么叫"下属的描述符?"
    * A:描述符是嵌套结构，举个例子，最简单的如下：

        ![描述符结构](https://chenlongos.com/Phytium-Car/assert/ch2-1_1.png)

        具体的来说，[资料可以看这里](https://www.usbzh.com/article/detail-22.html)

        在我们的代码中，这些描述符都被编写为结构体，[在这里](https://gitlab.eduxiji.net/T202412799992620/project2210132-232991/tree/phytium_pi_dev/crates/driver_usb/src/host/usb/descriptors)
* Slot号：简而言之，为了方便起见，设备在内存中的抽象应当与物理上的结构无关，因此我们会为每个设备分配一个Slot Id来区别他们。
* 传输包大小：即端点单次传输所能返回的数据量，这一部分的实现细节在硬件层，我们只需要配置这个字段即可。
* 多线程不完善：谁来把这个问题修一下？主要问题在于中断系统没修好，导致无法定时中断进行调度。

对于set_configuration这个方法，实际上，这个方法的命名有少许偏差，该方法不仅仅是设置了设备的配置，还进行了设备的初始化(如端点的配置)

## 驱动部分

在进行了以上步骤后，xhci部分的主要初始化逻辑(包括设备的初始化)就算是完成了，接下来让我们来看看驱动的部分

在继续之前，读者需要了解以下概念。

### 概念

* TRB：在xhci中，无论是对于主机的命令，还是对于设备的命令/数据交互，均通过TRB来完成，TRB是信息传输的最小单元，承载TRB的是TRB环，而[TRB的种类](https://chenlongos.com/Phytium-Car/assert/extensible-host-controler-interface-usb-xhci.pdf#page=208)与TRB环的种类对应。
* TRB环：顾名思义，是个环形队列，其逻辑结构由[XHCI规范-4.9章](https://chenlongos.com/Phytium-Car/assert/extensible-host-controler-interface-usb-xhci.pdf#page=166)给出，具体的来说，有三种环
  * 控制环(Control Ring)-**在主机视角看来**，只有主机拥有一个控制环，控制环的作用是发送[Command TRB](https://chenlongos.com/Phytium-Car/assert/extensible-host-controler-interface-usb-xhci.pdf#page=223)以改变主机的行为
  * 事件环(Event Ring)-同样的，一般来说仅有一个事件环，这个事件环归主机所有。当主机/设备的状态更新，或一次TRB传输完成/异常时，便会向事件环内传入[Event TRB](https://chenlongos.com/Phytium-Car/assert/extensible-host-controler-interface-usb-xhci.pdf#page=465)以向Xhci驱动告知该事件，Xhci驱动可以以轮询的方式主动获取通知，或是以中断的方式被动获取，在获得了对应的事件TRB后，驱动应当选择合适的处理函数
  * 传输环(Transfer Ring)-与上面的两个TRB环不同，设备的每一个端点都对应着一个传输环，传输环负责向设备发送请求，设备的回复则会传入事件环中，其对应着[Transfer TRB](https://chenlongos.com/Phytium-Car/assert/extensible-host-controler-interface-usb-xhci.pdf#page=210)。
    * 传输环比较特殊，正是传输环连接着USB协议与XHCI规范。传输环所容纳的Transfer TRB正是上文所述的四种传输的载体
* [HID协议](https://chenlongos.com/Phytium-Car/assert/hid1_11.pdf)：事实上，USB协议如同OSI模型一般，也是分层的结构，这一点从[USB规范](https://chenlongos.com/Phytium-Car/assert/[USB3.2协议规范]USB%203.2%20Revision%201.1.pdf)的目录中就可以看出，而HID则是属于上层建筑，其建立在USB协议所定义的四种传输类型的基础之上，具体的来讲，是建立在中断传输与控制传输的基础之上。

### 驱动部分-代码

由于目前仅作为Demo用途，因此我们实际上的代码并没有严格的遵循开头的那张图，相反的，我们[非常简陋但直接的的这么做](https://gitlab.eduxiji.net/T202412799992620/project2210132-232991/blob/phytium_pi_dev/apps/usb/src/main.rs#L55):

以下代码能正确执行的前提在于：你只插了一个鼠标，并且正确的修改了代码以适配了你的鼠标
可能出现的问题包括但不限于：

* 你的鼠标是个无线鼠标——理论上能够正常工作
  * 无线鼠标的接收器往往并不只有一个鼠标的配置，事实上，无线接收器在主机端看来是一个复合设备，同时具有鼠标/键盘/手柄等设备的Interface，这些描述符的布局因设备而异，因此请确保代码正确的配置了你的无线接收器
  * 小建议：你可以尝试修改linux的代码，使其能够打印出发送的TRB，并将其编译后在qemu上运行(当然，你得在qemu中挂载你的设备-运行时挂载，不要启动时挂载)来确定你到底要怎么修改TRB以配置你的鼠标
* 你的鼠标是个有线鼠标——我们希望他正常工作
  * 我们最初驱动刚写出来时支持有线鼠标，但在修改成支持无线鼠标后，由于同上的理由，鉴于我们尚未测试过目前使用有线鼠标的情况，我们同样无法保证兼容性。如果有线鼠标不能用，你可以试试将代码checkout到[这次提交](https://gitlab.eduxiji.net/T202412799992620/project2210132-232991/commit/f27af4ffe9de3d32df0b13bababbafcbe99b6a49)
  * 好在有线鼠标通常就只是有线鼠标，不会出现多接口描述符的情况。

```rust
//...
    usb.poll().unwrap();

    let mut device_list = usb.device_list();

    let mut hid = device_list.pop().unwrap(); //只取出第一个设备

    hid.test_hid_mouse().unwrap(); //并强行将其当作USB鼠标进行测试-这当然不安全，代码框架仍然需要进一步完善。
    println!("test done!");
//...
```

再次强调，这段代码仅作为临时展示用途，因此写的比较简陋

接下来让我们看看test_hid_mouse这个函数做了什么，基本上来说，这个函数做了所有USB鼠标驱动所需要做的事情：

```rust
//...
    pub fn test_hid_mouse(&mut self) -> Result {
        debug!("test hid");

        self.controller.lock().trace_dump_output_ctx(self.slot_id);

        let endpoint_in = self //获取最靠后的已启用端点-所有的，在当前Interface下包含的端点都已在先前启用，而USB鼠标通常就只有一个In方向的端点
            .configs
            .get(self.current_config)
            .unwrap()
            .interfaces
            .get(self.current_interface)
            .unwrap()
            .endpoints
            .last()
            .unwrap()
            .endpoint_address as usize;

        debug!("current testing endpoint address:{endpoint_in}");

        self.set_configuration()?;  //告知设备我们选择的配置
        self.set_interface()?;      //同样的，告知设备选择的interface

        debug!("reading HID report descriptors");

        //interface包含三个字段:class-设备类型，subclass-设备子类型，protocol-设备所用协议，在这里我们确保所连接到的设备是个HID设备
        if self.current_interface().data.interface_class != 3 {
            debug!("not hid");
            return Ok(());
        }

        let protocol = self.current_interface().data.interface_protocol;
        if self.current_interface().data.interface_subclass == 1 // subclass==1，意味着这是一个USB鼠标
        && protocol > 0//Hid设备有一种叫做"Bootable device"的子类，这类设备标志着他可以在系统引导前(即bios中)向bios发送更多可以用于分析的信息。
         {//但我们现在是在系统里，因此设置关闭多余的信息发送
            debug!("set protocol");

            self.control_transfer_out(
                0,
                ENDPOINT_OUT | REQUEST_TYPE_CLASS | RECIPIENT_INTERFACE,
                0x0B,
                if protocol == 2 { 1 } else { 0 },
                self.current_interface().data.interface_number as _,
                &[],
            )?;
        }

        // 设置回报率：引用hid规范，当回报率设置为0时，设备仅在检查到改变后进行一次传输，这正是中断传输名字的由来
        // 然而，多数设备默认都是将回报率设为0的，因此我们不需要手动设置。如果你的设备出现了异常，解除下面的注释并正确配置控制传输请求
        // "正确"指的是按照linux内核中dump出来的TRB的抓包结果来写！
        // debug!("set idle");
        // self.control_transfer_out(
        //     0,
        //     ENDPOINT_OUT | REQUEST_TYPE_CLASS | RECIPIENT_INTERFACE,
        //     HID_SET_IDLE,
        //     0x00,
        //     0,
        //     &[],
        // )?;

        //====================================================获取报表描述符====================================================
        //这里代码可能存在一些问题，有时候并没有正确的获取到报表描述符，但是我们可以直接根据标准的USB鼠标报表描述符来解析鼠标发过来的报文
        debug!("request feature report");
        let data = self.control_transfer_in(
            0,
            ENDPOINT_IN | REQUEST_TYPE_STANDARD | RECIPIENT_INTERFACE,
            REQUEST_GET_DESCRIPTOR,
            DescriptorType::HIDReport.forLowBit(0).bits(),
            self.current_interface().data.interface_number as _,
            256,
        )?;
        let descriptor_size = 256;
        debug!("descriptor_size {}", descriptor_size);

        let size = get_hid_record_size(&data, HID_REPORT_TYPE_FEATURE);
        if size <= 0 {
            debug!("Skipping Feature Report readout (None detected)");
        } else {
            debug!("Reading Feature Report (length {})...", size);

            let report_buffer = self.control_transfer_in(
                0,
                ENDPOINT_IN | REQUEST_TYPE_CLASS | RECIPIENT_INTERFACE,
                HID_GET_REPORT,
                (HID_REPORT_TYPE_FEATURE << 8) | 0,
                self.current_interface().data.interface_number as _,
                size as _,
            )?;
        }

        let size = get_hid_record_size(&data, HID_REPORT_TYPE_INPUT);

        if (size <= 0) {
            debug!("Skipping Input Report readout (None detected)");
        } else {
            debug!("Reading Input Report (length {})...", size);
            let report_buffer = self.control_transfer_in(
                0,
                ENDPOINT_IN | REQUEST_TYPE_CLASS | RECIPIENT_INTERFACE,
                HID_GET_REPORT,
                ((HID_REPORT_TYPE_INPUT << 8) | 0x00),
                0,
                size as _,
            )?;

            debug!("got buffer: {:?}", report_buffer);
            self.report_parser = {
                let try_from = ReportDescriptor::try_from(&report_buffer); //根据报表描述符创建报文解析器
                if try_from.is_ok() {
                    debug!("successed convert report descriptor!");
                } else {
                    debug!("convert failed,{:?}", try_from);
                }
                try_from.ok()
            };
        //=========================================================================================================

            // 设置返回的报文类型-同样的，如果你的设备出了问题，考虑是否要进行这一步
            // debug!("set report!");
            // self.control_transfer_in(
            //     0,
            //     ENDPOINT_IN | REQUEST_TYPE_CLASS | RECIPIENT_INTERFACE,
            //     HID_SET_REPORT,
            //     0, //wtf, captured from usblyzer, todo: analyze report descriptor to set this value for more device
            //     0,
            //     0, //no data control transfer
            // )?;

            debug!(
                "==============================================\nTesting interrupt read using endpoint {:#X}...",
                endpoint_in
            );

            // self.controller.lock().debug_dump_output_ctx(self.slot_id);

            //在进行正式的传输之前，我们需要将传输环的翻转位全部置一，以确保传输的正常运行，原因及细节详见下文
            self.controller
                .lock()
                .prepare_transfer_normal(self, ep_num_to_dci(endpoint_in));

            //TODO: resolve report descriptor 
            let size = 8;  //硬编码是坏习惯，这个值理应是min(端点最大单次传输量，报表描述符所描述的报文大小)，但是在这里，我们将其硬编码成8
            //关于为什么，请参见下文的解释


            debug!("report size is {size}");
            loop {
                let report_buffer = self.interrupt_in(endpoint_in, size as _)?; //获取一次中断传输-在设备返回报文之前，方法会一直阻塞在这里，这就是中断传输的由来，

                self.report_parser //如果报文解析器正常工作，那么就用报文解析器来解读报文
                    .as_ref()
                    .inspect(|parser| {
                        let input = parser.find_input_report(&report_buffer).unwrap();
                        input.fields().iter().for_each(|field| match field {
                            Field::Variable(var) => {
                                let val: u32 = var.extract_u32(&report_buffer).unwrap();
                                debug!("Field {:?} is of value {}", field, val);
                            }
                            Field::Array(arr) => {
                                let vals: Vec<u32> = arr.extract_u32(&report_buffer).unwrap();
                                debug!("Field {:?} has values {:?}", field, vals);
                            }
                            Field::Constant(_) => {
                                debug!("Field {:?} is <padding data>", field);
                            }
                        });
                    })
                    .or_else(|| {
                        trace!("got report: {:?}", report_buffer);
                        fallback_solve_hid_mouse_report(&report_buffer); //否则就使用标准的报文解析
                        None
                    });
            }
        }

        Ok(())
    }
//...
```

想必读者读到这里，心中已经有了很多疑惑：

* Q：为什么这么多被注释掉的代码？
  * A：我们也是从零开始摸索着写的，最初我们有参考USBlyzer的抓包结果(在windows上),后来又参考了linux的驱动初始化流程，同时也参考了很多其他系统，都是各有各的写法，于是就留下了这么多奇怪的东西。总之，现在已经能正常工作了，但以防万一我们还是将用不到的代码留着。
* Q：报文是如何被解析的？
  * A：[报表描述符](https://www.usbzh.com/article/detail-48.html)以Input字段做分隔，UseagePage表述当前段描述的内容，举个例子，这是我们做实验时用到的鼠标的报表描述符：

  ```log
    Interface 0 HID Report Descriptor Mouse
  
    Item Tag (Value) Raw Data                                   //左边是解读结果，右侧hex值为实际上拿到手的原始数据
    Usage Page (Generic Desktop) 05 01  
    Usage (Mouse) 09 02  
    Collection (Application) A1 01  
        Usage (Pointer) 09 01  //Pointer-鼠标指针
        Collection (Physical) A1 00  
            Usage Page (Button) 05 09                           //Button-这一块描述的是按键的解析方式
            Usage Minimum (Button 1) 19 01                      //最多可包含16个按钮，至少包含一个按钮
            Usage Maximum (Button 16) 29 10                         //按钮次序按位排列，就约定来说，一般约定bit0是左键，bit1是右键，bit2是中键
            Logical Minimum (0) 15 00                           //逻辑上的最小值-对应的位为0表示没有被按下
            Logical Maximum (1) 25 01                           //逻辑上的最大值-对应的位为1表示按钮被按下
            Report Count (16) 95 10                             //报告数量——重复16次——也就是一共有16个这样的按钮
            Report Size (1) 75 01                               //报告大小-1bit，16个报告*1bit=16位-即这个Page所描述的信息占用了报文的2个字节
            Input (Data,Var,Abs,NWrp,Lin,Pref,NNul,Bit) 81 02  
            Usage Page (Generic Desktop) 05 01                  //Generic Desktop-这里描述的是鼠标指针运动的解析方式
            Logical Minimum (-32767) 16 01 80                   //最小值为-32767
            Logical Maximum (32767) 26 FF 7F                    //最大值为 32767
            Report Size (16) 75 10                              //单个报告大小-16bit
            Report Count (2) 95 02                              //重复两次
            Usage (X) 09 30                                     //首先第一次是X方向的运动量
            Usage (Y) 09 31                                     //然后第二次是Y方向的运动量
            //综上得知，这一页描述的是鼠标运动，有XY两个方向，每个方向的运动量范围为从-32767到32767，为i16大小(两个字节)，两个方向加起来-两个i16，即占用了报文的4个字节
            Input (Data,Var,Rel,NWrp,Lin,Pref,NNul,Bit) 81 06   
            Logical Minimum (-127) 15 81                        //最小值为-127
            Logical Maximum (127) 25 7F                         //最大值为127
            Report Size (8) 75 08                               //占用8个bit大小
            Report Count (1) 95 01                              //不重复，只有一次
            Usage (Wheel) 09 38                                 //哦，原来是描述了鼠标滚轮啊!  ——所以Usage并不一定要出现在开头
            Input (Data,Var,Rel,NWrp,Lin,Pref,NNul,Bit) 81 06  
            Usage Page (Consumer Devices) 05 0C                 
            Usage (AC Pan) 0A 38 02                             //AC Pan，回报率切换按钮
            Report Count (1) 95 01                              
            Input (Data,Var,Rel,NWrp,Lin,Pref,NNul,Bit) 81 06  
        End Collection C0  
    End Collection C0  
  ```

