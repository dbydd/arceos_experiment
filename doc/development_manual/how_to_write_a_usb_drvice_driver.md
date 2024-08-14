# 在当前框架下，如何编写一个USB设备驱动？

## 1. 创建两个结构体，实现以下trait:
```rust
pub trait USBSystemDriverModule<'a, O>: Send + Sync
where
    O: PlatformAbstractions,
{
    fn should_active(
        &self,
        independent_dev: &DriverIndependentDeviceInstance<O>,
        config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    ) -> Option<Vec<Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>>>>;

    fn preload_module(&self);
}

pub trait USBSystemDriverModuleInstance<'a, O>: Send + Sync
where
    O: PlatformAbstractions,
{
    fn prepare_for_drive(&mut self) -> Option<Vec<URB<'a, O>>>;

    fn gather_urb(&mut self) -> Option<Vec<URB<'a, O>>>;

    fn receive_complete_event(&mut self, ucb: UCB<O>);
}
```

* 其中，*USBSystemDriverModule*是驱动模块，其负责创建驱动设备的实例，需注意的有以下几点：
    * should_active可以返回多个驱动设备实例，因为一个设备可实现多个Interface，这些Interface可能属于同一上层协议
    * should_active还承担过滤功能，如果设备不适用于该模块，直接返回None即可
    * preload_module会在驱动模块被完全加载前调用一次
* 其中，*USBSystemDriverModuleInstance*是驱动设备实例，是Interface的实现，需注意以下几点：
  * 出于安全考虑，驱动设备实例应尽量少的直接使用系统调用，而是使用事件系统进行间接调用
  * prepare_for_drive会在驱动设备刚刚被创建时调用一次，做对应Interface的初始化
  * gather_urb与receive_complete_event是一对，URB与UCB也会成对出现。发出去几个URB就会收到几次UCB
  * Vec\<URB\>中的URB会按下标从小到大逐个执行

## 2. 注册驱动模块：
* 我们支持动态注册，只需要在认为合适的时候将你的模块像这样注册进去就行：
```rust
//crates/driver_usb/src/usb/mod.rs#L47
            self.managed_modules.load_driver(Box::new(
                universal_drivers::hid_drivers::hid_mouse::HidMouseDriverModule,
            ));

```
## EXT. 没有我想要的事件，如何自定义一个？
* 请直接修改代码，事件的分发是模式匹配的，事件数量的多少不影响效率，因此事件不嫌多，只怕不够详细。只需要这样做就能创建一个事件：
```rust
pub enum USBSystemEvent {
    MouseEvent(MouseEvent),
    //在这里加上新的枚举，如:
    ExampleNewEvent(NewEventData) //作为样例
}

#[derive(Debug)]
pub struct MouseEvent {
    pub dx: isize,
    pub dy: isize,
    pub left: bool,
    pub right: bool,
    pub middle: bool,
    pub wheel: isize,
}

pub struct NewEventData {//如果你的事件需要传递数据，那就创建个承载数据的容器。
    //...fields
}

```

然后只需要在系统抽象层的实例中对该事件所述的控制分支写对应的处理就行