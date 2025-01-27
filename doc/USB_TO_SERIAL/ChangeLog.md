# ChangeLog

## unreleased  v 0.0.0

### Added

### Changed

### Removed

### Fixed

### Question

## 2025-1-16   v 0.0.0 usb-serial-dev

### Added

- 签出一个新的usb-serial-dev分支，用于修改代码，usb-camera-base分支用于与主仓库同步，文档都编写在usb-camera-base分支的doc中。
- `crates\driver_usb\src\usb\universal_drivers`中新建cdc_drivers目录。其中新建cdc_serial.rs和mod.rs。用于开发usb转串口的主要代码。
- cdc__serial.rs中新建“驱动模块”`CdcSerialDriverModule`和“驱动设备”`CdcSerialDriver`。
- cdc_serial.rs中为“驱动设备”`CdcSerialDriver`中实现`USBSystemDriverModuleInstance`特征的三个方法，未实现具体代码。
- cdc_serial.rs中为“驱动模块”`CdcSerialDriverModule`实现`USBSystemDriverModule`特征的两个方法，未实现具体代码。
- mod.rs中声明cdc_serial模块。
- doc目录中添加ChangeLog.md，即本文件。
- `crates\driver_usb\src\usb\mod.rs`中`USBDriverSystem`的init方法中添加对`USBSystemDriverModule`驱动模块的加载。

### Changed

### Removed

### Fixed

### Question

- `cdc_serial.rs`引入`descriptors::{desc_device::StandardUSBDeviceClassCode, desc_endpoint::Endpoint}`，这个描述符需要修改吗。

