# HT32F52352 USB CDC 详细分析 - Embassy 实现指南

## 目标
本文档提供完整的 USB CDC (Communication Device Class) 引导序列、数据流和寄存器时序分析，专门针对 Embassy 实现中遇到的问题。

## 概述

USB CDC 实现涉及两个关键接口：
- **通信接口 (Communication Interface)**: 控制和状态管理
- **数据接口 (Data Interface)**: 实际数据传输

与简单的 USB 设备不同，CDC 需要复杂的枚举序列和特定的描述符结构。

---

## 1. USB CDC 枚举序列详细时序

### 1.1 硬件连接阶段 (0-100ms)

```c
时间点: T0 - 设备连接
┌─────────────────────────────────────────────────┐
│ 1. 主机检测到设备连接 (D+ 上拉)                    │
│ 2. 主机等待设备稳定 (100ms debounce)              │
│ 3. 主机发送 USB RESET (SE0 状态 10-20ms)          │
└─────────────────────────────────────────────────┘

寄存器操作:
USB->CSR |= USBCSR_DPPUEN;              // 启用 D+ 上拉
// 等待主机检测...
// 主机发送 RESET 信号
```

**Embassy 关键点**: 确保在连接前完成所有端点配置。

### 1.2 USB 复位和地址分配 (100-200ms)

```c
时间点: T0+100ms - USB 复位中断
┌─────────────────────────────────────────────────┐
│ USB 复位中断处理:                                │
│ 1. 清除所有端点配置                              │
│ 2. 重置设备状态为 DEFAULT                        │
│ 3. 配置 EP0 (64字节控制端点)                     │
│ 4. 启用 EP0 中断                                 │
└─────────────────────────────────────────────────┘

寄存器操作:
// 复位中断处理
if (USB->ISR & USBISR_URSTIF) {
    // 1. 清除 CSR (保留 D+ 上拉)
    USB->CSR &= USBCSR_DPPUEN;

    // 2. 重置端点内存分配
    epmem_next = 8;  // EP0 使用前8字节

    // 3. 配置 EP0
    USB->EP[0].CFGR = USBEPnCFGR_EPEN | (0 << 24);  // 控制端点，地址0
    USB->EP[0].IER = USBEPnIER_SDRXIE | USBEPnIER_ODRXIE | USBEPnIER_IDTXIE;

    // 4. 启用基础中断
    USB->IER = USBIER_UGIE | USBIER_SOFIE | USBIER_URSTIE |
               USBIER_RSMIE | USBIER_SUSPIE | USBIER_EP0IE;

    // 清除中断标志
    USB->ISR = USBISR_URSTIF;
}
```

### 1.3 设备描述符请求 (200-250ms)

```c
时间点: T0+200ms - 第一个 SETUP 包
┌─────────────────────────────────────────────────┐
│ 主机请求: GET_DESCRIPTOR (Device)                 │
│ bmRequestType: 0x80 (Device-to-Host)             │
│ bRequest: 0x06 (GET_DESCRIPTOR)                  │
│ wValue: 0x0100 (Device Descriptor)               │
│ wIndex: 0x0000                                   │
│ wLength: 0x0012 (18 bytes)                       │
└─────────────────────────────────────────────────┘

寄存器时序:
// SETUP 数据接收中断
if (USB->EP[0].ISR & USBEPnISR_SDRXIF) {
    // 1. 从 USB SRAM 读取 SETUP 包 (8字节)
    uint32_t *setup_ram = (uint32_t*)USB_SRAM_BASE;
    uint8_t setup[8];
    for (int i = 0; i < 8; i += 4) {
        uint32_t word = setup_ram[i/4];
        setup[i+0] = word >> 0;
        setup[i+1] = word >> 8;
        setup[i+2] = word >> 16;
        setup[i+3] = word >> 24;
    }

    // 2. 解析请求
    if (setup[0] == 0x80 && setup[1] == 0x06 && setup[3] == 0x01) {
        // GET_DESCRIPTOR Device
        // 准备发送设备描述符
        device_descriptor_response();
    }

    USB->EP[0].ISR = USBEPnISR_SDRXIF;
}
```

**关键 CDC 设备描述符**:
```c
static const uint8_t device_descriptor[18] = {
    0x12,        // bLength
    0x01,        // bDescriptorType (Device)
    0x00, 0x02,  // bcdUSB (USB 2.0)
    0x02,        // bDeviceClass (CDC)
    0x02,        // bDeviceSubClass (ACM)
    0x00,        // bDeviceProtocol
    0x40,        // bMaxPacketSize0 (64)
    0xd9, 0x04,  // idVendor
    0x0d, 0xf0,  // idProduct
    0x00, 0x01,  // bcdDevice
    0x01,        // iManufacturer
    0x02,        // iProduct
    0x03,        // iSerialNumber
    0x01         // bNumConfigurations
};
```

### 1.4 配置描述符请求 (250-300ms)

这是 CDC 最复杂的部分，需要返回完整的接口关联描述符。

```c
时间点: T0+250ms - GET_DESCRIPTOR (Configuration)
┌─────────────────────────────────────────────────┐
│ 主机请求完整配置信息                              │
│ wLength: 0x0043 (67字节) - CDC 典型长度           │
└─────────────────────────────────────────────────┘

完整 CDC 配置描述符结构:
┌─────────────────────────────────────────────────┐
│ 配置描述符 (9字节)                                │
│ 接口关联描述符 (8字节) - CDC 特有                 │
│ 通信接口描述符 (9字节)                           │
│ CDC 头功能描述符 (5字节)                         │
│ CDC ACM 功能描述符 (4字节)                       │
│ CDC 联合功能描述符 (5字节)                       │
│ 通信端点描述符 (7字节) - EP1 IN                  │
│ 数据接口描述符 (9字节)                           │
│ 数据端点描述符 (7字节) - EP2 OUT                 │
│ 数据端点描述符 (7字节) - EP2 IN                  │
└─────────────────────────────────────────────────┘
总计: 67字节
```

**完整 CDC 配置描述符**:
```c
static const uint8_t cdc_config_descriptor[] = {
    // 配置描述符 (9字节)
    0x09, 0x02, 0x43, 0x00, 0x02, 0x01, 0x00, 0xC0, 0x32,

    // 接口关联描述符 (8字节) - 关键！
    0x08, 0x0B, 0x00, 0x02, 0x02, 0x02, 0x00, 0x00,
    //      IAD   首接口 接口数 CDC  ACM

    // 通信接口描述符 (9字节)
    0x09, 0x04, 0x00, 0x00, 0x01, 0x02, 0x02, 0x00, 0x00,
    //      接口  接口0  备用0  1端点  CDC  ACM

    // CDC 头功能描述符 (5字节)
    0x05, 0x24, 0x00, 0x10, 0x01,
    //      CS接口 头      CDC1.1

    // CDC ACM 功能描述符 (4字节)
    0x04, 0x24, 0x02, 0x06,
    //      CS接口 ACM   能力位图

    // CDC 联合功能描述符 (5字节)
    0x05, 0x24, 0x06, 0x00, 0x01,
    //      CS接口 联合  主接口 从接口

    // 通信端点描述符 (7字节) - EP1 IN
    0x07, 0x05, 0x81, 0x03, 0x40, 0x00, 0x10,
    //      端点  EP1IN 中断   64字节   16ms

    // 数据接口描述符 (9字节)
    0x09, 0x04, 0x01, 0x00, 0x02, 0x0A, 0x00, 0x00, 0x00,
    //      接口  接口1  备用0  2端点  数据类

    // 数据端点描述符 (7字节) - EP2 OUT
    0x07, 0x05, 0x02, 0x02, 0x40, 0x00, 0x00,
    //      端点  EP2OUT 批量  64字节

    // 数据端点描述符 (7字节) - EP2 IN
    0x07, 0x05, 0x82, 0x02, 0x40, 0x00, 0x00
    //      端点  EP2IN  批量  64字节
};
```

### 1.5 SET_CONFIGURATION (300-350ms)

```c
时间点: T0+300ms - SET_CONFIGURATION
┌─────────────────────────────────────────────────┐
│ 主机选择配置1                                    │
│ bmRequestType: 0x00                              │
│ bRequest: 0x09 (SET_CONFIGURATION)               │
│ wValue: 0x0001 (Configuration 1)                 │
└─────────────────────────────────────────────────┘

寄存器操作 - 端点配置:
// EP1 IN (通信端点) - 中断端点
uint32_t ep1_cfgr = USBEPnCFGR_EPEN |        // 使能
                    (1 << 24) |               // 端点地址1
                    (0x40 << 0) |             // 缓冲区偏移
                    (64 << 10) |              // 缓冲区大小64字节
                    USBEPnCFGR_EPDIR;         // IN方向
USB->EP[1].CFGR = ep1_cfgr;
USB->EP[1].IER = USBEPnIER_IDTXIE;           // IN传输完成中断

// EP2 OUT (数据端点) - 批量端点
uint32_t ep2_out_cfgr = USBEPnCFGR_EPEN |    // 使能
                        (2 << 24) |           // 端点地址2
                        (0x80 << 0) |         // 缓冲区偏移
                        (64 << 10);           // 缓冲区大小64字节
USB->EP[2].CFGR = ep2_out_cfgr;
USB->EP[2].IER = USBEPnIER_ODRXIE;           // OUT接收中断

// EP2 IN (数据端点) - 批量端点
uint32_t ep2_in_cfgr = USBEPnCFGR_EPEN |     // 使能
                       (2 << 24) |            // 端点地址2
                       (0xC0 << 0) |          // 缓冲区偏移
                       (64 << 10) |           // 缓冲区大小64字节
                       USBEPnCFGR_EPDIR;      // IN方向
// 注意: EP2 IN 使用相同端点号，但不同缓冲区

// 使能端点中断
USB->IER |= USBIER_EP1IE | USBIER_EP2IE;
```

**Embassy 关键注意事项**:
1. 端点配置必须在 SET_CONFIGURATION 时完成
2. 缓冲区地址不能重叠
3. CDC 需要接口关联描述符 (IAD)

### 1.6 CDC 特定请求处理 (350-400ms)

CDC 设备会收到类特定的控制请求:

```c
时间点: T0+350ms - CDC 类请求
┌─────────────────────────────────────────────────┐
│ SET_LINE_CODING - 设置串口参数                   │
│ GET_LINE_CODING - 获取串口参数                   │
│ SET_CONTROL_LINE_STATE - 设置DTR/RTS             │
└─────────────────────────────────────────────────┘

typedef struct {
    uint32_t dwDTERate;     // 波特率 (如 115200)
    uint8_t bCharFormat;    // 停止位 (0=1位, 1=1.5位, 2=2位)
    uint8_t bParityType;    // 奇偶校验 (0=无, 1=奇, 2=偶)
    uint8_t bDataBits;      // 数据位数 (5,6,7,8)
} __packed cdc_line_coding_t;

// SET_LINE_CODING 处理
if (setup[0] == 0x21 && setup[1] == 0x20) {
    // bmRequestType: 0x21 (Host-to-Device, Class, Interface)
    // bRequest: 0x20 (SET_LINE_CODING)

    // 准备接收7字节line coding数据
    USB->EP[0].TCR = 7;
    USB->EP[0].CSR &= ~USBEPnCSR_NAKRX;  // 允许接收

    // 在OUT数据接收中断中处理实际数据
}

// GET_LINE_CODING 处理
if (setup[0] == 0xA1 && setup[1] == 0x21) {
    // 发送当前line coding
    static cdc_line_coding_t line_coding = {
        .dwDTERate = 115200,
        .bCharFormat = 0,    // 1 stop bit
        .bParityType = 0,    // No parity
        .bDataBits = 8       // 8 data bits
    };

    // 写入EP0 IN缓冲区并发送
    send_ep0_data((uint8_t*)&line_coding, sizeof(line_coding));
}

// SET_CONTROL_LINE_STATE 处理
if (setup[0] == 0x21 && setup[1] == 0x22) {
    // wValue 包含控制线状态:
    // bit 0: DTR (Data Terminal Ready)
    // bit 1: RTS (Request To Send)
    uint16_t control_state = (setup[3] << 8) | setup[2];

    bool dtr = control_state & 0x01;
    bool rts = control_state & 0x02;

    // 发送状态ACK
    send_ep0_status();
}
```

---

## 2. 数据流详细分析

### 2.1 OUT 数据接收流程 (主机→设备)

```c
数据接收完整时序:
┌─────────────────────────────────────────────────┐
│ 1. 主机发送数据到EP2 OUT                         │
│ 2. USB硬件接收数据到USB SRAM                    │
│ 3. 触发ODRXIF中断                                │
│ 4. 软件从USB SRAM复制到应用缓冲区                │
│ 5. 清除NAK，准备接收下一个包                     │
└─────────────────────────────────────────────────┘

寄存器操作细节:
// EP2 OUT 数据接收中断
if (USB->EP[2].ISR & USBEPnISR_ODRXIF) {
    // 1. 读取接收字节数
    size_t rx_count = USB->EP[2].TCR & USBEPnTCR_TCNT;

    // 2. 计算USB SRAM地址
    uint32_t cfgr = USB->EP[2].CFGR;
    uint32_t buffer_addr = cfgr & USBEPnCFGR_EPBUFA;  // 缓冲区地址
    volatile uint32_t *usb_sram = (volatile uint32_t*)(USB_SRAM_BASE + buffer_addr);

    // 3. 从USB SRAM复制数据
    for (size_t i = 0; i < rx_count; i += 4) {
        uint32_t word = usb_sram[i / 4];
        if (i + 0 < rx_count) app_buffer[rx_pos + i + 0] = (word >> 0) & 0xFF;
        if (i + 1 < rx_count) app_buffer[rx_pos + i + 1] = (word >> 8) & 0xFF;
        if (i + 2 < rx_count) app_buffer[rx_pos + i + 2] = (word >> 16) & 0xFF;
        if (i + 3 < rx_count) app_buffer[rx_pos + i + 3] = (word >> 24) & 0xFF;
    }

    rx_pos += rx_count;

    // 4. 清除中断标志
    USB->EP[2].ISR = USBEPnISR_ODRXIF;

    // 5. 准备接收下一个包
    USB->EP[2].CSR &= ~USBEPnCSR_NAKRX;

    // 6. 通知应用层数据可用
    if (cdc_rx_callback) {
        cdc_rx_callback(app_buffer, rx_count);
    }
}
```

### 2.2 IN 数据发送流程 (设备→主机)

```c
数据发送完整时序:
┌─────────────────────────────────────────────────┐
│ 1. 应用层调用发送函数                            │
│ 2. 复制数据到USB SRAM                           │
│ 3. 设置TCR计数器                                 │
│ 4. 清除NAK开始传输                              │
│ 5. 等待IDTXIF中断                               │
│ 6. 处理多包传输(如果需要)                        │
└─────────────────────────────────────────────────┘

发送函数实现:
void cdc_send_data(const uint8_t *data, size_t length) {
    size_t remaining = length;
    size_t offset = 0;

    while (remaining > 0) {
        // 1. 计算本次发送字节数
        size_t packet_size = (remaining > 64) ? 64 : remaining;

        // 2. 等待EP2 IN就绪
        while (USB->EP[2].CSR & USBEPnCSR_NAKTX) {
            // EP正在发送，等待完成
        }

        // 3. 计算USB SRAM地址 (EP2 IN)
        uint32_t cfgr = USB->EP[2].CFGR;
        uint32_t buffer_addr = (cfgr & USBEPnCFGR_EPBUFA) +
                               ((cfgr >> 10) & 0x7F);  // IN缓冲区偏移
        volatile uint32_t *usb_sram = (volatile uint32_t*)(USB_SRAM_BASE + buffer_addr);

        // 4. 复制数据到USB SRAM (按32位对齐)
        for (size_t i = 0; i < packet_size; i += 4) {
            uint32_t word = 0;
            if (offset + i + 0 < length) word |= data[offset + i + 0] << 0;
            if (offset + i + 1 < length) word |= data[offset + i + 1] << 8;
            if (offset + i + 2 < length) word |= data[offset + i + 2] << 16;
            if (offset + i + 3 < length) word |= data[offset + i + 3] << 24;
            usb_sram[i / 4] = word;
        }

        // 5. 设置传输计数器并开始发送
        USB->EP[2].TCR = packet_size;
        USB->EP[2].CSR &= ~USBEPnCSR_NAKTX;  // 清除NAK开始传输

        remaining -= packet_size;
        offset += packet_size;

        // 6. 等待传输完成中断
        while (!(USB->EP[2].ISR & USBEPnISR_IDTXIF)) {
            // 等待传输完成
        }
        USB->EP[2].ISR = USBEPnISR_IDTXIF;  // 清除中断标志
    }

    // 7. 如果最后一个包正好64字节，发送零长度包
    if (length % 64 == 0) {
        USB->EP[2].TCR = 0;
        USB->EP[2].CSR &= ~USBEPnCSR_NAKTX;
        while (!(USB->EP[2].ISR & USBEPnISR_IDTXIF)) {}
        USB->EP[2].ISR = USBEPnISR_IDTXIF;
    }
}
```

---

## 3. Embassy 实现要点

### 3.1 Embassy USB 驱动结构差异

与 ChibiOS 相比，Embassy 使用异步模式：

```rust
// Embassy USB CDC 典型结构
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::{Builder, Config, UsbDevice};

// 1. USB 驱动初始化
let driver = // HT32 USB 驱动实例
let mut config = Config::new(0x04d9, 0xf00d);
config.manufacturer = Some("Holtek");
config.product = Some("HT32 CDC");
config.serial_number = Some("12345678");

// 2. CDC 类配置
let mut device_descriptor = [0; 256];
let mut config_descriptor = [0; 256];
let mut bos_descriptor = [0; 256];
let mut control_buf = [0; 64];

let mut builder = Builder::new(
    driver,
    config,
    &mut device_descriptor,
    &mut config_descriptor,
    &mut bos_descriptor,
    &mut control_buf,
);

// 3. CDC ACM 类实例化
let mut state = State::new();
let mut class = CdcAcmClass::new(&mut builder, &mut state, 64);

// 4. 构建设备
let mut usb = builder.build();

// 5. 异步运行循环
let usb_task = async {
    usb.run().await;
};

let echo_task = async {
    loop {
        class.wait_connection().await;

        // CDC 数据处理
        let mut buf = [0; 64];
        match class.read_packet(&mut buf).await {
            Ok(n) => {
                // 回显数据
                let _ = class.write_packet(&buf[..n]).await;
            }
            Err(_) => break,
        }
    }
};

// 并发运行任务
join(usb_task, echo_task).await;
```

### 3.2 关键实现差异

1. **异步vs同步**: Embassy 使用 async/await，不是中断驱动
2. **内存管理**: Embassy 自动管理 USB SRAM 分配
3. **端点配置**: Embassy 自动处理描述符生成

### 3.3 可能的 Embassy 实现问题

1. **时钟配置错误**:
```rust
// 确保 USB 时钟精确为 48MHz
let mut rcc = // 时钟配置
rcc.usb_clock_source(UsbClockSource::Pll);  // 144MHz / 3 = 48MHz
```

2. **端点缓冲区冲突**:
```rust
// Embassy 可能没有正确处理 HT32 的 USB SRAM 布局
// 需要检查端点缓冲区地址分配
```

3. **中断优先级问题**:
```rust
// 确保 USB 中断优先级正确
interrupt::enable(Interrupt::USB, Priority::P5);
```

---

## 4. 调试和验证步骤

### 4.1 硬件验证清单

```c
// 1. 时钟验证
assert(USB_CLOCK_FREQ == 48000000);

// 2. D+ 上拉验证
assert(USB->CSR & USBCSR_DPPUEN);

// 3. 端点配置验证
assert(USB->EP[0].CFGR & USBEPnCFGR_EPEN);  // EP0 使能
assert(USB->EP[1].CFGR & USBEPnCFGR_EPEN);  // EP1 使能
assert(USB->EP[2].CFGR & USBEPnCFGR_EPEN);  // EP2 使能

// 4. 中断使能验证
assert(USB->IER & USBIER_EP0IE);
assert(USB->IER & USBIER_EP1IE);
assert(USB->IER & USBIER_EP2IE);
```

### 4.2 枚举过程调试

```c
// 添加调试输出跟踪枚举过程
void debug_usb_setup(uint8_t *setup) {
    printf("SETUP: %02x %02x %04x %04x %04x\n",
           setup[0], setup[1],
           *(uint16_t*)&setup[2],
           *(uint16_t*)&setup[4],
           *(uint16_t*)&setup[6]);

    switch (setup[1]) {
        case 0x06: printf("  -> GET_DESCRIPTOR\n"); break;
        case 0x09: printf("  -> SET_CONFIGURATION\n"); break;
        case 0x20: printf("  -> SET_LINE_CODING\n"); break;
        case 0x21: printf("  -> GET_LINE_CODING\n"); break;
        case 0x22: printf("  -> SET_CONTROL_LINE_STATE\n"); break;
        default: printf("  -> UNKNOWN REQUEST\n"); break;
    }
}

// 在 SETUP 中断处理中调用
if (USB->EP[0].ISR & USBEPnISR_SDRXIF) {
    read_setup_packet(setup_buffer);
    debug_usb_setup(setup_buffer);
    // ... 继续处理
}
```

### 4.3 数据传输验证

```c
// 验证 USB SRAM 访问
void verify_usb_sram_access() {
    volatile uint32_t *sram = (volatile uint32_t*)USB_SRAM_BASE;

    // 写入测试模式
    sram[0] = 0x12345678;
    assert(sram[0] == 0x12345678);

    sram[0] = 0xABCDEF00;
    assert(sram[0] == 0xABCDEF00);

    printf("USB SRAM 访问正常\n");
}

// 验证端点缓冲区地址
void verify_endpoint_buffers() {
    for (int ep = 0; ep <= 2; ep++) {
        uint32_t cfgr = USB->EP[ep].CFGR;
        uint32_t addr = cfgr & USBEPnCFGR_EPBUFA;
        uint32_t size = (cfgr >> 10) & 0x7F;

        printf("EP%d: addr=0x%03x, size=%d\n", ep, addr, size * 4);

        // 检查地址范围
        assert(addr < 0x400);  // 不能超过 USB SRAM 大小
        assert(addr + size * 4 <= 0x400);
    }
}
```

---

## 5. Embassy 特定的实现建议

### 5.1 HAL 层适配

如果 Embassy 没有直接的 HT32 USB 支持，需要实现底层驱动：

```rust
// 实现 Embassy USB 驱动特征
use embassy_usb_driver::{Driver, EndpointType, EndpointIn, EndpointOut};

struct Ht32UsbDriver {
    // HT32 USB 寄存器访问
}

impl Driver for Ht32UsbDriver {
    type EndpointOut = Ht32EndpointOut;
    type EndpointIn = Ht32EndpointIn;
    type ControlPipe = Ht32ControlPipe;
    type Bus = Ht32Bus;

    fn alloc_endpoint_in(
        &mut self,
        ep_type: EndpointType,
        max_packet_size: u16,
        interval: u8,
    ) -> Result<Self::EndpointIn, EndpointAllocError> {
        // 分配 HT32 IN 端点
        // 参考上面的端点配置代码
    }

    fn alloc_endpoint_out(
        &mut self,
        ep_type: EndpointType,
        max_packet_size: u16,
        interval: u8,
    ) -> Result<Self::EndpointOut, EndpointAllocError> {
        // 分配 HT32 OUT 端点
        // 参考上面的端点配置代码
    }
}
```

### 5.2 中断处理适配

```rust
// Embassy 中断处理
#[interrupt]
fn USB() {
    // 调用 Embassy USB 驱动的中断处理器
    Ht32UsbDriver::on_interrupt();
}

impl Ht32UsbDriver {
    fn on_interrupt() {
        let isr = USB.isr().read();

        if isr.urstif() {
            // USB 复位中断
            Self::handle_reset();
        }

        if isr.ep0if() {
            // EP0 中断
            Self::handle_ep0();
        }

        // 处理其他端点中断...
    }
}
```

这个详细的分析应该能帮助你识别 Embassy 实现中的具体问题。重点关注：
1. 时钟配置的准确性
2. USB SRAM 缓冲区地址分配
3. 端点配置的正确性
4. 中断处理的完整性
5. CDC 描述符的规范性

你可以根据这个分析对比你的 Embassy 实现，找出可能的问题所在。