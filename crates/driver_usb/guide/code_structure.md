# 代码结构导读

## 前情提要

在之前的代码结构导读中，我们介绍了位于分支"phytium_pi_port"上的代码。目前，随着 xhci 部分的结束，我们已经将代码重构并迁移到了"phytium_pi_dev"分支，最新的驱动架构能够应对更多复杂情况，但结构也发生了大的变化，因此，我们重新做一遍代码导读。

## 文件结构

```log
.
├── Cargo.lock
├── Cargo.toml
├── guide
│   ├── code_file_structure.log
│   ├── code_structure.md
│   └── quickstart_usb.md
└── src
    ├── addr.rs
    ├── ax
    │   └── mod.rs
    ├── device_types.rs
    ├── dma.rs
    ├── err.rs
    ├── host
    │   ├── device.rs
    │   ├── mod.rs
    │   └── xhci
    │       ├── context.rs
    │       ├── event.rs
    │       ├── mod.rs
    │       ├── registers.rs
    │       └── ring.rs
    └── lib.rs
6 directories, 18 files
```

## 入口

目前，我们暂时抛弃了原来的 cli 手动启动，而是将 usb 模块的引导做成了一个[app](../../../apps/usb/src/main.rs)：

```rust

#[derive(Clone)]
struct OsDepImp;

impl OsDep for OsDepImp { //我们优化了驱动的架构，将其重构为系统无关的库，因此我们需要引入对各个操作系统进行适配的抽象层，也就是说，我们将驱动需要操作系统做的事情抽象了出来，形成了这个trait："OsDep"
    type DMA = GlobalNoCacheAllocator;

    const PAGE_SIZE: usize = axalloc::PAGE_SIZE;
    fn dma_alloc(&self) -> Self::DMA { //就目前来说，我们仅仅只需要操作系统负责分配出No Cache的内存区域（DMA)即可。
        axalloc::global_no_cache_allocator()
    }
}

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    let phytium_cfg_id_0 = (0xffff_0000_31a0_8000, 48, 0);

    let config = USBHostConfig::new(
        phytium_cfg_id_0.0,
        phytium_cfg_id_0.1,
        phytium_cfg_id_0.2,
        OsDepImp {},
    );

    let usb = USBHost::new::<Xhci<_>>(config).unwrap();

    usb.poll();
}
```

让我们看看[USBHostConfig](../src/host/mod.rs)里有什么：

```rust
#[derive(Clone)]
pub struct USBHostConfig<O>
where O: OsDep
{
    pub(crate) base_addr: VirtAddr,
    pub(crate) irq_num: u32,      //中断系统相关：中断号。
    pub(crate) irq_priority: u32, //中断系统相关：中断优先级。注：目前中断系统尚未适配
    pub(crate) os: O //集成了操作系统相关操作的trait对象
}
```

接下来，是与我们曾经的代码逻辑相似的 USBHost(XHCI)初始化流程,也就是[USBHost::new](../src/host/xhci/mod.rs)方法,这部分所做事情与移植之前相差不大，请读者自行比对理解。

## 深入

让我们来看看最新的进展，在 USBHost 创建之后，就会被调用 poll 方法，这个方法的作用是进行设备的枚举：

```rust
    fn probe(&self) -> Result {
        let mut port_id_list = Vec::new();
        {
            let mut g = self.regs.lock();
            let regs = &mut g.regs;
            let port_len = regs.port_register_set.len();
            for i in 0..port_len {
                let portsc = &regs.port_register_set.read_volatile_at(i).portsc;
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

                port_id_list.push(i); //寻找所有连接上了设备的port
            }
        }
        for port_id in port_id_list {
            let slot = self.device_slot_assignment(port_id); //首先，是为设备分配slot
            self.address_device(slot, port_id);              //在这里，我们配置对应slot的上下文(context)，并请求xhci为设备设置地址
            self.set_ep0_packet_size(slot);                  //通过控制传输，获取准确的endpoint 0传输数据包大小
            self.setup_fetch_all_dev_desc(slot);             //在以上的配置完成后，获取设备的全部描述符。
        }

        self.dev_ctx
            .lock()
            .attached_set
            .iter_mut()
            .for_each(|dev| {}); //TODO: 为设备选择制定的配置，而后开始传输。目前我们先做一个HID设备（鼠标/键盘）
        Ok(())
    }
```

## 任务分解 1-设备描述符

设备描述符是设备所包含的描述信息，在这里，我们一次性获取所有的描述符信息，并在需要的时候获取对应条目的描述符

设备描述符有许多种类，不同的种类描述了不同的信息，比如 device 就可能会包含设备的厂家/设备的类型等信息，[参考](../src/host/usb/descriptors/mod.rs):

```rust
#[derive(ConstEnum, Copy, Clone, Debug)]
#[allow(non_camel_case_types)]
pub(crate) enum Type {
    //USB 1.1: 9.4 Standard Device Requests, Table 9-5. Descriptor Types
    Device = 1,
    Configuration = 2,
    String = 3,
    Interface = 4,
    Endpoint = 5,
    // USB 2.0: 9.4 Standard Device Requests, Table 9-5. Descriptor Types
    DeviceQualifier = 6,
    OtherSpeedConfiguration = 7,
    InterfacePower1 = 8,
    Hid = 0x21,
    HIDReport = 0x22,
    HIDPhysical = 0x23,
    // USB 3.0+: 9.4 Standard Device Requests, Table 9-5. Descriptor Types
    OTG = 0x09,
    Debug = 0x0a,
    InterfaceAssociation = 0x0b,
    Bos = 0x0f,
    DeviceCapability = 0x10,
    SuperSpeedEndpointCompanion = 0x30,
    SuperSpeedPlusIsochEndpointCompanion = 0x31,
}
```

这些类型，每一个都对应了不同的 Descriptor，每个 Descriptor 又有不同的数据结构，好在我们目前暂时不用全部实现，只实现需要的部分即可。我们所需要关心的代码位于这里,

```rust
impl Descriptor {
    pub(crate) fn from_slice(raw: &[u8]) -> Result<Self, Error> {
        assert_eq!(raw.len(), raw[0].into());
        match FromPrimitive::from_u8(raw[1]) {
            Some(t) => {
                let raw: *const [u8] = raw;
                match t {
                    // SAFETY: This operation is safe because the length of `raw` is equivalent to the
                    // one of the descriptor.
                    //todo 任务1：我们所需要的就是在这里补充描述符的创建,在setup_fetch_all_dev_desc方法中，这个方法会被调用多次以反序列化设备传输过来的描述符
                    Type::Device => Ok(Self::Device(unsafe { ptr::read(raw.cast()) })),
                    Type::Configuration => {
                        Ok(Self::Configuration(unsafe { ptr::read(raw.cast()) }))
                    }
                    Type::String => Ok(Self::Str),
                    Type::Interface => Ok(Self::Interface(unsafe { ptr::read(raw.cast()) })),
                    Type::Endpoint => Ok(Self::Endpoint(unsafe { ptr::read(raw.cast()) })),
                    Type::Hid => Ok(Self::Hid),
                    other => unimplemented!("please implement descriptor type:{:?}", other),
                }
            }
            None => Err(Error::UnrecognizedType(raw[1])),
        }
    }
}
```

额外的参考资料:

- [USB 中文网-关于设备描述符的部分](https://www.usbzh.com/article/detail-104.html)
- USB3.2 spec 文档：在资料附件中

## 任务分解 2：设备配置选择

在获取到了描述符后，我们要从设备提供的几种配置中选择一种来设置端点（endpoint），参考：[redox 的代码](https://github.com/redox-os/drivers/blob/master/xhcid/src/xhci/scheme.rs#L595)

这部分在我们的代码中应当位于[根据设备描述符查找驱动](../src/host/xhci/xhci_device.rs)时

## 任务分解 3-HID 驱动编写-键盘

先写一个键盘驱动，参考 [redox 代码](https://github.com/redox-os/drivers/tree/master/usbhidd)
