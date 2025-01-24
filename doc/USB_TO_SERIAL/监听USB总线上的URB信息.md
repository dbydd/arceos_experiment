# 在Linux上监听USB总线的URB信息
## 查看CH340的配置
CH340需要使用URB来进行配置，在Linux中可以直接读取USB总线上的URB信息。首先加载`usbmon`内核模块
```
sudo modprobe usbmon
```
然后使用`lsusb`查看本机连接的usb设备
```
hong@hong-Legion-Y7000-2019-PG0:~/下载$ lsusb
Bus 002 Device 001: ID 1d6b:0003 Linux Foundation 3.0 root hub
Bus 001 Device 006: ID 048d:c100 Integrated Technology Express, Inc. ITE Device(8910)
Bus 001 Device 005: ID 04f2:b604 Chicony Electronics Co., Ltd Integrated Camera (1280x720@30)
Bus 001 Device 016: ID 1a86:7523 QinHeng Electronics CH340 serial converter
Bus 001 Device 003: ID 320f:5088 Telink VGN S99 2.4G Dongle
Bus 001 Device 007: ID 8087:0aaa Intel Corp. Bluetooth 9460/9560 Jefferson Peak (JfP)
Bus 001 Device 011: ID 3554:f503 Compx VGN Mouse 2.4G Receiver
Bus 001 Device 001: ID 1d6b:0002 Linux Foundation 2.0 root hub

```
可以看到CH340是总线1的16号设备，vid:pid为1a86:7523。
## 读取USB总线1的URB信息
执行
`sudo cat /sys/kernel/debug/usb/usbmon/1u`
可以看到类似下面这样的数据（实际不是这个，下面这个是获取CH340设备描述符的URB）
```
ffff8998ea249480 2370066358 S Ci:1:025:0 s 80 06 0100 0000 0012 18 <
ffff8998ea249480 2370066562 C Ci:1:025:0 0 18 = 12011001 ff000008 861a2375 64020002 0001

```
解读一下：
以 ffff8998450ca600 1597763019 C Ii:1:003:1 0:1 8 = 00000000 00000000 这行为例：
1. ffff8998ea249480：这是 URB 的唯一标识符，用于在内核中追踪该请求块。
2. 2370066358 和 2370066562：分别是 URB 提交和完成的时间戳，单位通常是系统启动以来的毫秒数。
3. S 和 C：S 表示 URB 已提交（Submitted），C 表示 URB 已完成（Completed）。
4. Ci:1:025:0：C 代表控制传输（Control Transfer）；i 表示输入方向（从设备到主机）；1 是总线编号；025 是设备地址；0 是端点编号（控制传输通常使用端点 0）。
5. s 80 06 0100 0000 0012：这是控制传输的请求类型和参数。80 表示标准的输入请求；06 是 GET_DESCRIPTOR 请求；0100 表示请求的是设备描述符（0x01 为设备描述符类型）；0000 通常是语言 ID；0012 表示请求的描述符长度为 18 字节。
6. 18 < 和 18 = ...：前面的 18 < 表示请求的数据长度为 18 字节；后面的 18 = ... 表示实际接收到的数据长度为 18 字节，具体数据为 12011001 ff000008 861a2375 64020002 0001，这些数据包含了设备的基本信息，如描述符长度、USB 版本、厂商 ID、产品 ID 等。
## 只读取CH340有关的URB
首先断开CH340与主机的连接，执行
`sudo cat /sys/kernel/debug/usb/usbmon/1u | grep 1:025`
**注意这个022,每抽插一次CH340这个设备编号会递增一个**
这时候什么也看不到，因为CH340还没连接。接下来插上CH340查看Linux和CH340之间发送了哪些URB。
```
ffff8998ea249480 2370066358 S Ci:1:025:0 s 80 06 0100 0000 0012 18 <
ffff8998ea249480 2370066562 C Ci:1:025:0 0 18 = 12011001 ff000008 861a2375 64020002 0001
ffff8998ea249480 2370066588 S Ci:1:025:0 s 80 06 0200 0000 0009 9 <
ffff8998ea249480 2370066730 C Ci:1:025:0 0 9 = 09022700 01010080 31
ffff8998ea249480 2370066737 S Ci:1:025:0 s 80 06 0200 0000 0027 39 <
ffff8998ea249480 2370066980 C Ci:1:025:0 0 39 = 09022700 01010080 31090400 0003ff01 02000705 82022000 00070502 02200000
ffff8998ea249480 2370066996 S Ci:1:025:0 s 80 06 0300 0000 00ff 255 <
ffff8998ea249480 2370067112 C Ci:1:025:0 0 4 = 04030904
ffff8998ea249480 2370067129 S Ci:1:025:0 s 80 06 0302 0409 00ff 255 <
ffff8998ea249480 2370067408 C Ci:1:025:0 0 22 = 16035500 53004200 20005300 65007200 69006100 6c00
ffff8998ea249480 2370071220 S Co:1:025:0 s 00 09 0001 0000 0000 0
ffff8998ea249480 2370071261 C Co:1:025:0 0 0
ffff8998b30b7600 2370077553 S Ci:1:025:0 s 80 06 03ee 0000 0400 1024 <
ffff8998b30b7600 2370077699 C Ci:1:025:0 -32 0
ffff8998e914ccc0 2370091909 S Ci:1:025:0 s 80 06 03ee 0000 0400 1024 <
ffff8998e914ccc0 2370092100 C Ci:1:025:0 -32 0
ffff8998e914ccc0 2370677518 S Ci:1:025:0 s c0 5f 0000 0000 0002 2 <
ffff8998e914ccc0 2370677593 C Ci:1:025:0 0 2 = 3100
ffff8998e914ccc0 2370677645 S Co:1:025:0 s 40 a1 0000 0000 0000 0
ffff8998e914ccc0 2370677709 C Co:1:025:0 0 0
ffff8998e914ccc0 2370677749 S Co:1:025:0 s 40 9a 1312 d982 0000 0
ffff8998e914ccc0 2370677833 C Co:1:025:0 0 0
ffff8998e914ccc0 2370677854 S Co:1:025:0 s 40 9a 0f2c 0007 0000 0
ffff8998e914ccc0 2370677930 C Co:1:025:0 0 0
ffff8998e914ccc0 2370677935 S Ci:1:025:0 s c0 95 2518 0000 0002 2 <
ffff8998e914ccc0 2370678028 C Ci:1:025:0 0 2 = c300
ffff8998e914ccc0 2370678111 S Ci:1:025:0 s c0 95 0706 0000 0002 2 <
ffff8998e914ccc0 2370678279 C Ci:1:025:0 0 2 = ffee
ffff8998e914ccc0 2370678283 S Co:1:025:0 s 40 9a 2727 0000 0000 0
ffff8998e914ccc0 2370678379 C Co:1:025:0 0 0
ffff8998e914ccc0 2370678383 S Co:1:025:0 s 40 a1 c39c b282 0000 0
ffff8998e914ccc0 2370678446 C Co:1:025:0 0 0
ffff8998e914ccc0 2370678449 S Co:1:025:0 s 40 9a 0f2c 0008 0000 0
ffff8998e914ccc0 2370678483 C Co:1:025:0 0 0
ffff8998e914ccc0 2370678486 S Co:1:025:0 s 40 9a 2727 0000 0000 0
ffff8998e914ccc0 2370678526 C Co:1:025:0 0 0
ffff8998ea249480 2370678529 S Ii:1:025:1 -115:1 8 <
ffff8998ea249180 2370678532 S Bi:1:025:2 -115 32 <
ffff8998ea249840 2370678534 S Bi:1:025:2 -115 32 <
ffff8998ea2499c0 2370678537 S Bi:1:025:2 -115 32 <
ffff8998ea249900 2370678540 S Bi:1:025:2 -115 32 <
ffff8998e914ccc0 2370678555 S Co:1:025:0 s 40 a1 c39c cc83 0000 0
ffff8998e914ccc0 2370678596 C Co:1:025:0 0 0
ffff8998e914ccc0 2370678599 S Co:1:025:0 s 40 9a 0f2c 0007 0000 0
ffff8998e914ccc0 2370678637 C Co:1:025:0 0 0
ffff8998e914ccc0 2370678640 S Co:1:025:0 s 40 a4 00df 0000 0000 0
ffff8998e914ccc0 2370678681 C Co:1:025:0 0 0
ffff8998e914ccc0 2370678684 S Co:1:025:0 s 40 9a 2727 0000 0000 0
ffff8998e914ccc0 2370678722 C Co:1:025:0 0 0
ffff8998e914ccc0 2370678728 S Co:1:025:0 s 40 a4 009f 0000 0000 0
ffff8998e914ccc0 2370678763 C Co:1:025:0 0 0
```
然后抽出CH340查看有那些URB
```
ffff8998ea249180 2443686993 C Bi:1:025:2 -71 0
ffff8998ea249840 2443687173 C Bi:1:025:2 -71 0
ffff8998ea2499c0 2443687283 C Bi:1:025:2 -71 0
ffff8998ea249900 2443687396 C Bi:1:025:2 -71 0
ffff8998ea249480 2443687560 C Ii:1:025:1 -108:1 0
```
## 关于CH340的端点说明
从之前给出的 CH340 设备描述符可知，该设备有三个端点，下面是每个端点的用途：

### 端点 1（`0x82` - EP 2 IN）
```plaintext
Endpoint Descriptor:
  bLength                 7
  bDescriptorType         5
  bEndpointAddress     0x82  EP 2 IN
  bmAttributes            2
    Transfer Type            Bulk
    Synch Type               None
    Usage Type               Data
  wMaxPacketSize     0x0020  1x 32 bytes
  bInterval               0
```
- **端点地址**：`0x82` 代表这是端点 2 且方向为输入（IN），即主机从设备接收数据会使用这个端点。
- **传输类型**：`Transfer Type` 为 `Bulk`（批量传输）。批量传输适用于大量数据的可靠传输，通常不要求数据实时到达，但对数据准确性要求高。在 CH340 里，这个端点可能用于传输串口转换后的大量数据，比如从串口接收到的数据发送给主机。
- **同步类型**：`Synch Type` 为 `None`，意味着此端点在传输数据时不需要额外的同步机制。
- **使用类型**：`Usage Type` 为 `Data`，表明该端点专门用于数据传输。
- **最大数据包大小**：`wMaxPacketSize` 为 `0x0020`（32 字节），即该端点一次最多能传输 32 字节的数据。
- **轮询间隔**：`bInterval` 为 0，对于批量传输端点，此值通常设为 0。

### 端点 2（`0x02` - EP 2 OUT）
```plaintext
Endpoint Descriptor:
  bLength                 7
  bDescriptorType         5
  bEndpointAddress     0x02  EP 2 OUT
  bmAttributes            2
    Transfer Type            Bulk
    Synch Type               None
    Usage Type               Data
  wMaxPacketSize     0x0020  1x 32 bytes
  bInterval               0
```
- **端点地址**：`0x02` 表示这是端点 2 且方向为输出（OUT），也就是主机向设备发送数据会使用这个端点。
- **传输类型**：同样是 `Bulk`（批量传输），用于大量数据的可靠传输。在 CH340 中，该端点可能用于主机将需要通过串口发送的数据传递给设备。
- **同步类型**：`None`，不需要额外同步机制。
- **使用类型**：`Data`，用于数据传输。
- **最大数据包大小**：`0x0020`（32 字节），一次最多传输 32 字节数据。
- **轮询间隔**：`0`，符合批量传输端点的特点。

### 端点 3（`0x81` - EP 1 IN）
```plaintext
Endpoint Descriptor:
  bLength                 7
  bDescriptorType         5
  bEndpointAddress     0x81  EP 1 IN
  bmAttributes            3
    Transfer Type            Interrupt
    Synch Type               None
    Usage Type               Data
  wMaxPacketSize     0x0008  1x 8 bytes
  bInterval               1
```
- **端点地址**：`0x81` 代表这是端点 1 且方向为输入（IN），主机从设备接收数据使用此端点。
- **传输类型**：`Transfer Type` 为 `Interrupt`（中断传输）。中断传输适用于少量数据的实时传输，设备有紧急数据需要发送时会主动发起中断请求，主机响应后接收数据。在 CH340 里，这个端点可能用于传输一些状态信息或事件通知，比如串口状态变化等。
- **同步类型**：`None`，无需额外同步机制。
- **使用类型**：`Data`，用于数据传输。
- **最大数据包大小**：`wMaxPacketSize` 为 `0x0008`（8 字节），一次最多传输 8 字节数据。
- **轮询间隔**：`bInterval` 为 1，意味着主机每隔 1 个帧（对于 USB 1.1 来说是 1ms）会轮询一次该端点，查看是否有数据。
