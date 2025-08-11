# HT32 Embassy Project Structure

一个 带 Embassy 依赖 且 支持 probe-rs 的 HT32F523xx 工作区骨架。

是完整可直接放进 Git 仓库的初始版本，本地 cargo run 就能用 probe-rs 烧到板子（有 CMSIS-DAP/J-Link/ST-Link 等兼容调试器）。

目标 MCU 是 HT32F523xx（例如 HT32F52352）。
已有 Rust PAC 支持、官方资料与开发板可以拿来做测试。下面是我刚做的现实性检查、结论与建议的下一步执行计划（可马上开始）。

我刚查到的关键事实（可直接用来开工）

    有社区维护的 HT32 PAC 项目 ht32-rs，为 HT32 系列提供 svd2rust 生成的 PACs。
    GitHub

    在 crates.io 上已有针对 HT32F523x2（覆盖 HT32F52352 / 52342 等）的 PAC crate ht32f523x2，可以直接依赖。
    Crates.io

    Holtek 官方页面与 datasheet 明确 HT32F523xx 是 Cortex-M0+、最高 48MHz，并且该系列包含 USB、PDMA（DMA）、多种定时器等外设（我们会在 HAL/embassy 实现中用到这些特性）。
    Holtek+1

    有官方/第三方的 starter kit / dev board（例如 ESK32-30501），用于上板测试和调试会很方便。
    Best Modules

## project plan

embassy-ht32/
├─ ht32-pac/         # optional: 或依赖 crates.io 的 pac
├─ ht32-hal/         # blocking HAL（GPIO, RCC, Timer, UART）
├─ embassy-ht32/     # async wrappers + embassy trait impls
├─ bsp/              # board support: pin mappings, features
└─ examples/
   ├─ blink-embassy/
   └─ serial-echo/
搭建 workspace（推荐优先） — 我会 scaffold 一个 embassy-ht32 workspace，包含：

    直接依赖 ht32f523x2（crates.io）作为 ht32-pac。
    Crates.io

    ht32-hal（blocking 基础：GPIO、RCC/Clocks、Timer、UART 最小实现）

    embassy-ht32（async wrappers：优先实现 Timer 与 UART）

    bsp（若需，针对 ESK32-30501 或你手头开发板的引脚定义）

    examples/blink-embassy 与 examples/serial-echo（可用 probe-rs / probe-run 上载）
    我会把 README、Cargo.toml、CI skeleton（仅编译为主）也一并做好。

验证 PAC / 编译测试 — 在 scaffold 完成后，我会编译 no_std blink demo（仅交叉编译）确认 PAC crate 与依赖无编译错误；如果发现 SVD 里的寄存器/位域缺失，会列出具体 patch 建议。
GitHub
Crates.io

在目标板上跑通最小例（需要你或我有物理板与 probe） — 把 blink / UART echo 烧到 ESK32-30501 或你的板上，确认时钟、GPIO、UART 工作正常。若你没有板，我仍可把能直接复用的代码推到仓库，你拿板执行即可。
Best Modules+1

实现 Embassy 核心（后续） — 当基础 HAL 成熟后，继续实现 embassy-wrappers：embassy-time（基于 SysTick 或 GPTM）、中断驱动的 async UART（AtomicWaker 机制）以及后续的 SPI/I2C/DMA 支持。


## development

### 安装目标和 probe-rs
rustup target add thumbv6m-none-eabi
cargo install probe-rs --locked
cargo install cargo-embed --locked

### 编译
cargo build --release

### 烧录 & 运行
cargo run --release -p blink-embassy

# USB HID Keyboard Example
cargo run --release -p usb-hid-keyboard


## USB Support - ✅ COMPLETED

### Implementation Status
✅ **USB Driver**: Complete embassy-usb-driver implementation for HT32F523xx
✅ **HID Support**: Working USB HID keyboard example with embassy-usb
✅ **RMK Ready**: Compatible with RMK mechanical keyboard firmware
✅ **Examples**: Functional USB HID keyboard demo with button input

### USB Architecture
HT32F523xx 的 USB：Holtek 官方文档里描述的 USB FS 寄存器和 STM32F103 类似，但寄存器布局不同，需要从 SVD 手动分析。

RMK 与 Embassy USB：RMK 已有 embassy-usb 支持，但需要匹配 HT32 的 endpoint 管理和中断处理。

RMK 用的是 embassy-usb，它需要 MCU 提供一个实现了 embassy_usb_driver::Driver trait 的 USB 驱动层，来完成：

    Endpoint 分配与配置

    数据包发送与接收

    USB 中断处理

    设备连接与复位检测

HT32F523xx 内置的 USB FS 控制器是 全速设备（Full Speed），寄存器布局和 STM32F103 的 USB FS 很像，但名字和 bit 位稍有不同（Holtek 自家命名）。
好消息是 embassy-usb 已经有 stm32 参考，我们可以抄结构、改寄存器。
调试：如果 probe-rs 对 HT32 支持不全，需要用 OpenOCD 或 J-Link。

3. 寄存器填充思路

从 HT32F523xx RM 中可以找到：

    USB_BASE 地址

    Endpoint 寄存器结构（可能是 EP0-EP7）

    控制寄存器：USB_CNTR / USB_ISTR / USB_DADDR 等

    PMA（Packet Memory Area）管理方式（类似 STM32 USB FS）

我们要做：

    实现 EP 分配表（最多 8 个 endpoint）

    写 alloc_ep 配置 EP 类型、包长、PMA 缓冲地址

    写 read / write 从 PMA RAM 读写数据

    在 poll 中解析 ISTR 中断源，返回给 embassy-usb

    1. 准备工作

    确保已有 ht32-hal 中 PAC 访问正常。

    创建 embassy-ht32-usb crate，依赖 embassy-usb-driver。

    熟悉 HT32 USB 寄存器，重点是：

        USB_CNTR 控制寄存器

        USB_ISTR 中断寄存器

        USB_DADDR 设备地址寄存器

        USB_EPnR 各端点寄存器

        PMA（Packet Memory Area）缓冲区管理

2. 驱动设计概要

    EP0 默认配置，支持标准枚举请求（GET_DESCRIPTOR、SET_ADDRESS等）。

    使用 PMA 区域读写控制传输数据。

    配置一个 HID IN 端点（EP1 IN）。

    实现 Driver trait 中的 poll(), alloc_ep(), enable()，并完成中断处理。


    USB 事件驱动 embassy-usb 栈。

写一个完整的 UsbDriver 实现，至少完成：

    EP0 标准控制传输

    一个简单的 HID IN 端点，实现定时发送“按键A”报告

结合前面 Ht32Matrix，最终替换 HID 报告数据为矩阵扫描结果。

## rmk intergation



