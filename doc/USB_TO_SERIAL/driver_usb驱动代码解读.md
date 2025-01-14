# driver_usb驱动代码

[源码](https://github.com/arceos-usb/arceos_experiment/tree/usb-camera-base/crates/driver_usb)

[前面同学留下的教程](https://github.com/arceos-usb/arceos_experiment/blob/usb-camera-base/doc/development_manual/how_to_write_a_usb_drvice_driver.md)

# 驱动包结构

```
.
├── Cargo.lock
├── Cargo.toml
└── src
    ├── abstractions
    │   ├── dma.rs
    │   ├── event
    │   │   └── mod.rs
    │   └── mod.rs
    ├── err.rs
    ├── glue
    │   ├── driver_independent_device_instance.rs
    │   ├── mod.rs
    │   └── ucb.rs
    ├── host
    │   ├── data_structures
    │   │   ├── host_controllers
    │   │   │   ├── mod.rs
    │   │   │   └── xhci
    │   │   │       ├── context.rs
    │   │   │       ├── event_ring.rs
    │   │   │       ├── logtxt.txt
    │   │   │       ├── mod.rs
    │   │   │       └── ring.rs
    │   │   └── mod.rs
    │   └── mod.rs
    ├── lib.rs
    └── usb
        ├── descriptors
        │   ├── desc_configuration.rs
        │   ├── desc_device.rs
        │   ├── desc_endpoint.rs
        │   ├── desc_hid.rs
        │   ├── desc_interface.rs
        │   ├── desc_report.rs
        │   ├── desc_str.rs
        │   ├── desc_uvc
        │   │   ├── mod.rs
        │   │   ├── uvc_endpoints.rs
        │   │   └── uvc_interfaces.rs
        │   ├── mod.rs
        │   ├── parser.rs
        │   └── topological_desc.rs
        ├── drivers
        │   ├── driverapi.rs
        │   └── mod.rs
        ├── mod.rs
        ├── operation
        │   └── mod.rs
        ├── trasnfer
        │   ├── control.rs
        │   ├── endpoints
        │   │   └── mod.rs
        │   ├── interrupt.rs
        │   └── mod.rs
        ├── universal_drivers
        │   ├── hid_drivers
        │   │   ├── hid_keyboard.rs
        │   │   ├── hid_mouse.rs
        │   │   ├── mod.rs
        │   │   └── temp_mouse_report_parser.rs
        │   ├── mod.rs
        │   └── uvc_drivers
        │       ├── generic_uvc.rs
        │       └── mod.rs
        └── urb.rs
```

# 功能分析

这是一个完整的USB协议栈实现，包含：

1. 底层主机控制器驱动。
2. USB核心协议的实现。
3. 设备描述符的处理。
4. 通用设备驱动，截至目前有HID和UVC设备。
5. 传输层实现。

**如果要添加具体设备（USB转串口设备）的驱动，应该是属于通用设备驱动。**接下来看下HID和UVC设备的实现。

# HID设备

包括键盘和鼠标驱动。以鼠标驱动为例子(键盘很类似)：

![1736774237705](.\driver_usb驱动代码\1736774237705.png)

其中最主要的是两个结构体`HidMouseDriverModule` `HidMouseDriver`，分别实现了两个trait。

## `HidMouseDriverModule`实现`USBSystemDriverModule`

驱动模块应该实现这个特征，包含两个函数：

1. `should_active`判断是否应该激活此驱动。返回一个Option，要么是None，要么是封装了实现了`USBSystemDriverModuleInstance`特征的一个结构体，应该就对应`HidMouseDriver`

   1. 检查设备是否匹配此驱动。
   2. 解析设备描述符。
   3. 收集必要的端点信息。
   4. 创建驱动实例。
   5. 处理复合设备的情况。（复合设备(Composite Device)是指一个物理 USB 设备包含多个功能的设备。比如有的设备只有一个USB插入，但是可以有鼠标、键盘还有存储功能，例如游戏手柄）

   ```
   impl<'a, O> USBSystemDriverModule<'a, O> for HidMouseDriverModule 
   where O: PlatformAbstractions + 'static 
   {
       fn should_active(
           &self,
           independent_dev: &DriverIndependentDeviceInstance<O>,
           config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
       ) -> Option<Vec<Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>>>> {
           // 1. 检查设备描述符
           if let MightBeInited::Inited(desc) = &*independent_dev.descriptors {
               let device = desc.device.first().unwrap();
               
               // 2. 匹配设备类型
               match (
                   StandardUSBDeviceClassCode::from(device.data.class),
                   USBHidDeviceSubClassCode::from_u8(device.data.subclass),
                   device.data.protocol,
               ) {
                   // 3. 处理直接的 HID 鼠标设备
                   (
                       StandardUSBDeviceClassCode::HID,
                       Some(USBHidDeviceSubClassCode::Mouse),
                       bootable,
                   ) => {
                       return Some(vec![HidMouseDriver::new_and_init(
                           independent_dev.slotid,
                           bootable,
                           device.endpoints,
                           config.clone(),
                           independent_dev.interface_val,
                           independent_dev.configuration_val,
                       )]);
                   }
                   
                   // 4. 处理复合设备中的 HID 接口
                   (StandardUSBDeviceClassCode::ReferInterfaceDescriptor, _, _) => {
                       // 遍历配置和接口
                       Some(self.match_interface_descriptor(
                           device, 
                           independent_dev, 
                           config
                       ))
                   }
                   
                   _ => None,
               }
           }
           None
       }
   }
   ```

   **USB转串口需要做的是：**

   - **检查设备描述符**
   - **匹配设备类型**
     - **USB转串口设备，返回一个驱动实例**
     - **默认返回None**

2. `preload_module`预加载模块，demo暂时只用打印一条语句。

## `HidMouseDriver`实现了`USBSystemDriverModuleInstance`

驱动实例应该实现这个特征，包含三个函数：

1. `prepare_for_drive`返回一个`Option<Vec<URB<'a, O>>>` 类型。`URB (USB Request Block) `是 USB 传输的基本单位，用于描述一个 USB 传输请求。这个方法push了四个URB到数组中，分别用于：

   1. 设置配置
   2. 设置接口
   3. 获取报告描述符
   4. 准备端点，端点(Endpoint)是 USB 设备上进行数据传输的通道。每个端点都有特定的用途和特性。

2. `gather_urb`在发送状态下执行

   1. 准备接收缓冲区
   2. 获取缓冲区并创建URB

   最后返回一个URB。

3. `receive_complete_event`。UCB (USB Completion Block) 是 USB 传输完成后的状态信息块，用于表示传输的结果和相关信息。根据UCB的状态来确认传输状态。

   1. 成功完成传输，处理接收到的数据，解析并发送鼠标事件，切换到发送状态，准备下一次传输。
   2. 传输错误。
   3. 其它完成代码，打印完成语句，切换发送状态。

在往上一层，看下`crates\driver_usb\src\usb`的功能。

# driver_usb中的`usb`模块

代码结构：

```
.
├── descriptors
│   ├── desc_configuration.rs
│   ├── desc_device.rs
│   ├── desc_endpoint.rs
│   ├── desc_hid.rs
│   ├── desc_interface.rs
│   ├── desc_report.rs
│   ├── desc_str.rs
│   ├── desc_uvc
│   │   ├── mod.rs
│   │   ├── uvc_endpoints.rs
│   │   └── uvc_interfaces.rs
│   ├── mod.rs
│   ├── parser.rs
│   └── topological_desc.rs
├── drivers
│   ├── driverapi.rs
│   └── mod.rs
├── mod.rs
├── operation
│   └── mod.rs
├── trasnfer
│   ├── control.rs
│   ├── endpoints
│   │   └── mod.rs
│   ├── interrupt.rs
│   └── mod.rs
├── universal_drivers
│   ├── hid_drivers
│   │   ├── hid_keyboard.rs
│   │   ├── hid_mouse.rs
│   │   ├── mod.rs
│   │   └── temp_mouse_report_parser.rs
│   ├── mod.rs
│   ├── serial_drivers
│   └── uvc_drivers
│       ├── generic_uvc.rs
│       └── mod.rs
└── urb.rs
```

- descriptors中定义了包括设备、端点、解析器等多个对象的描述符。

- driver中定义了驱动实例和驱动模块需要实现的特征，声明了一个可以存放若干驱动模块的容器。

- operation中定义了两个操作，配置和“额外步骤”。

- transfer包含三个模块

  - 中断：定义InterruptTransfer中断传输结构体。

  - 端点：直接[use const_enum::ConstEnum;](https://github.com/dbydd/const-enum-new.git)

  - 控制：

    - 定义ControlTransfer控制传输结构体，表示一个USB控制传输请求。
    - 定义bmRequestType结构体，表示请求类型的具体字段。

接下来再看driver_usb下面的其它模块

# driver_usb中的abstractions

driver_usb这个mod主要是为了适配不同的操作系统和硬件来做的，内核只要实现OSAbstractions和HALAbstractions就可以跑这个驱动。之前ArceOS应该已经适配过了，所以这个具体实现暂时不用操心。

# driver_usb中的glue

主要提供了ucb和driver_independent_device_instance两个子模块。

1. ucb：定义ucb结构体，实际里面只有一个返回的完成代码。
2. driver_independent_device_instance，接受插槽 ID 和控制器作为参数，用于管理 USB 设备的状态和描述符，便于在 USB 驱动系统中进行设备的初始化、配置和操作。

```
pub struct DriverIndependentDeviceInstance<O>
where
    O: PlatformAbstractions,
{
    pub slotid: usize,
    pub configuration_val: usize,
    pub interface_val: usize,
    pub current_alternative_interface_value: usize,
    pub descriptors: Arc<MightBeInited<TopologicalUSBDescriptorRoot>>,
    pub controller: ControllerArc<O>,
}
```

- 通过 controller 字段，结构体能够与 USB 控制器进行交互，执行控制传输和其他操作。
- 通过 descriptors 字段，结构体能够存储和管理设备的描述符信息，支持设备的配置和接口切换。

# driver_usb中的host

这实际是USB主机系统的实现。

- USB 系统配置: 提供了 USB 系统配置的构造函数。
- USB 主机系统: 定义了 USB 主机系统的结构体，包含控制器和配置。
- 控制器初始化: 提供了初始化控制器的方法。
- 设备探测: 实现了探测连接的 USB 设备的方法。
- 控制传输和设备配置: 提供了执行控制传输和配置设备的方法。
- URB 请求处理: 处理 URB 请求，根据请求类型调用相应的控制器方法。
- 待办事项处理: 处理待办事项列表，确保请求的完成事件被正确处理

其中关键的传输部分都是调用了controller的实现。xhci调用了外部地三方库，`crates\driver_usb\src\host\data_structures\host_controllers`提供了xhci的接口。提供的功能如

- new: 创建一个新的控制器实例，接受 USB 系统配置作为参数。
- init: 初始化控制器。
- probe: 探测连接的 USB 设备，返回设备的插槽 ID 列表。
- control_transfer: 执行控制传输请求，返回 URB 的结果。
- interrupt_transfer: 执行中断传输请求，返回 URB 的结果。
- configure_device: 配置 USB 设备，返回 URB 的结果。
- extra_step: 执行额外的步骤，返回 URB 的结果。
- device_slot_assignment: 获取设备插槽分配信息。
- address_device: 为设备分配地址，接受插槽 ID 和端口 ID。
- control_fetch_control_point_packet_size: 获取控制点数据包大小。
- set_ep0_packet_size: 设置端点 0 的数据包大小。



