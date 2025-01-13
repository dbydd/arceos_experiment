# USB转串口驱动开发

# 需求

**在现有ArceOS基础上开发USB转串口驱动，使得基于 USB 接口的设备能够在计算机系统中模拟串口功能，进而实现与其他串口设备或应用程序进行数据通信，支持飞腾派开发板CH340芯片的USB转串口。**

# 参考资料

[Linux USB转串口驱动 ch341.c](https://github.com/torvalds/linux/blob/master/drivers/usb/serial/ch341.c)

[Linux USB serial核心提供传输功能](https://github.com/torvalds/linux/blob/master/drivers/usb/serial/generic.c)

[ArceO包含USB驱动的分支](https://github.com/arceos-usb/arceos_experiment)

[南京沁恒微电子提供的USB转串口程序](https://www.wch.cn/downloads/CH341SER_LINUX_ZIP.html)

[CH340技术手册](https://www.wch.cn/downloads/CH340DS1_PDF.html)

# 驱动功能

* [ ] **抽象USB转串口设备（CH340）的数据结构**
* [ ] **定义USB转串口驱动结构**
* [ ] **USB驱动层**
  * [ ] **端口探测**
  * [ ] **端口初始化**
  * [ ] **端口移除**
* [ ] **主机硬件层**
  * [ ] **硬件初始化**
  * [ ] **配置串口参数**
  * [ ] **发送数据**
  * [ ] **接收数据**

# 开发参考路线

[参考资料](https://chenlongos.com/raspi4-with-arceos-doc/chapter_0.html)

* **PCIe总线初始化，可以读取USB设备ID。通过pid和vid来判断设备类型。**
* **为USB设备分配内存空间，以便进行信号和数据传输。**
* **xhci主机控制器的初始化。（xhci把计算机通过 PCIe 总线传来的指令、数据等按照合适的方式转换为 USB 设备能够识别的信号，同时也能把 USB 设备传来的数据、状态信息等转换为计算机内部总线可以处理的形式，从而实现硬件层面上的有效通信。支持USB3.0，兼容老版本USB。）**
* **枚举检测设备，为设备分配地址，获取设备的各类描述符。**
* **解析设备配置，加载相应的驱动程序，这里应该就是调用USB转串口的驱动。**
* **USB转串口的驱动实现，大致功能应如**[驱动功能](https://file+.vscode-resource.vscode-cdn.net/c:/Users/hong/Desktop/project/internship/arceos_experiment/doc/USB_TO_SERIAL/#%20%E9%A9%B1%E5%8A%A8%E5%8A%9F%E8%83%BD)
* **完成技术需求，撰写技术总结文档**

# 接下来的工作

* [X] **跑通飞腾派硬件烧录ArceOS流程。**
* [ ] **参考Linux USB驱动源码，查看具体USB转串口实现细节。**
* [ ] **参考ArceOS现有USB驱动源码，总结代码逻辑和设计思路。**
* [ ] **设计ArceOS USB传串口驱动框架和测试方法。**
