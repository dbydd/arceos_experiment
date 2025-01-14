# 使用串口传输usb-hid到飞腾派记录

使用串口传输到开发板中，物理连接在文档中说的很清楚了，但使用minicom需要Linux环境，这里我将Winodws的USB转串口设备分享到wsl中，然后从wsl中启动minicom。

## 分享USB转串口设备

参考[资料](https://learn.microsoft.com/zh-cn/windows/wsl/connect-usb)，注意在管理员终端执行这些命令。

把usb转串口设备分享到wsl后，wsl需要探测驱动，修改权限。

```
sudo modprobe ch341
sudo modprobe usbserial
sudo modprobe cp210x
sudo modprobe ftdi_sio
sudo chmod a+rw /dev/ttyUSB0
```

## 安装必要软件

```
pip3 install pyserial
pip3 install xmodem
apt install minicom
```

## 配置minicom

需要修改选择的串口为ttyUSB0。[参考](https://cloud.tencent.com/developer/article/2070964)。其他的不用改。

## 进入uboot

上电后，按空格键可以进入uboot，有时候可能是键盘没通电，这种情况可以按一下reset，再按空格就可以。其实任意键都可以，我按的空格。

## 清除启动命令

在uboot中清除原本镜像的启动命令

```
setenv bootcmd '';saveenv;
```

之后每次进入uboot都只用按一下reset了。

接下来退出wsl。

## 编译内核文件进行串口传输

克隆[源码仓库](https://github.com/Jasonhonghh/arceos_experiment)之后在根目录打开wsl，执行

```
make A=apps/usb-hid PLATFORM=aarch64-phytium-pi LOG=trace chainboot
```

在按一次reset键，可以进行传输，并且自动进入uboot界面

## 运行内核

接着在uboot的命令行中执行

```
go 0x90100000
```

可以看到如下的界面

![202501141820438](https://3ec93ca.webp.li/202501141820438.png)
