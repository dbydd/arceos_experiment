# 系统移植成功，并启动命令行界面运行截图:
![运行截图](./doc/figures/arceos_cli.png)


# USB系统正常运行，并获取鼠标输出运行日志
[运行日志](./doc/resources/res/usb-hid.log)

以上日志部分截图：
![usb-hid](./doc/resources/res/usb-hid.png)


## 承诺：以上案例与输出均可以通过[doc](./doc/)文件夹下的复现指南复现
---
# I2C系统正常运行，并进行数据读写
注：i2c代码位于usb-learnlings1分支
![i2c_1](./doc/resources/i2c/i2c_1.png)
![i2c_2](./doc/resources/i2c/i2c_2.png)

特别说明：该案例位于usb_learnings1分支，并未形成独立的app而是以crates（系统组件）的方式存在，若要复现此案例，应当运行apps/cli这个程序，我们对此分支上的cli程序做了一些更改，使其新增了i2c相关的命令。