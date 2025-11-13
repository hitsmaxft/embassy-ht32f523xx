# HT32F52352 USB CDC 寄存器时序图

## 完整枚举和数据传输时序

```
时间轴 (ms)  |  事件                    |  寄存器操作                           |  期望值/状态
=========================================================================================================
T+0         |  设备连接                |  USB->CSR |= USBCSR_DPPUEN           |  0x400 (D+ 上拉)
            |                         |                                      |
T+100       |  主机检测并复位           |  <- 主机发送 RESET                    |  SE0 状态 10-20ms
            |                         |                                      |
T+120       |  USB 复位中断            |  USB->ISR & USBISR_URSTIF           |  0x0004
            |                         |  USB->CSR &= USBCSR_DPPUEN           |  0x400 (清除其他位)
            |                         |  epmem_next = 8                      |  EP0 占用前8字节
            |                         |  USB->EP[0].CFGR = 0x80000000        |  EP0 使能，地址0
            |                         |  USB->EP[0].IER = 0x212              |  SETUP+OUT+IN中断
            |                         |  USB->IER = 0x113E                   |  基础+EP0中断
            |                         |  USB->ISR = 0x0004                   |  清除复位中断
            |                         |                                      |
T+150       |  主机请求设备描述符       |  <- SETUP: 80 06 00 01 00 00 12 00   |  GET_DESCRIPTOR Device
            |                         |  USB->EP[0].ISR & 0x200              |  SDRXIF 中断
            |                         |  从 USB_SRAM_BASE 读取 8 字节         |
            |                         |  USB->EP[0].ISR = 0x200              |  清除 SETUP 中断
            |                         |                                      |
T+151       |  发送设备描述符           |  复制18字节到 USB_SRAM_BASE          |  设备描述符数据
            |                         |  USB->EP[0].TCR = 18                 |  设置传输计数
            |                         |  USB->EP[0].CSR &= ~0x02             |  清除 NAKTX
            |                         |                                      |
T+152       |  IN 传输完成             |  USB->EP[0].ISR & 0x010              |  IDTXIF 中断
            |                         |  USB->EP[0].ISR = 0x010              |  清除 IN 中断
            |                         |                                      |
T+155       |  状态阶段 OUT            |  <- 主机发送零长度 OUT                |
            |                         |  USB->EP[0].ISR & 0x002              |  ODRXIF 中断
            |                         |  USB->EP[0].TCR >> 16 == 0           |  零长度包
            |                         |  USB->EP[0].ISR = 0x002              |  清除 OUT 中断
            |                         |                                      |
T+200       |  主机请求配置描述符       |  <- SETUP: 80 06 00 02 00 00 43 00   |  GET_DESCRIPTOR Config
            |                         |  USB->EP[0].ISR & 0x200              |  SDRXIF 中断
            |                         |  解析请求: wLength = 67               |
            |                         |  USB->EP[0].ISR = 0x200              |  清除 SETUP 中断
            |                         |                                      |
T+201       |  发送配置描述符 (第1包)   |  复制64字节到 USB_SRAM_BASE          |  配置描述符前64字节
            |                         |  USB->EP[0].TCR = 64                 |  第一个数据包
            |                         |  USB->EP[0].CSR &= ~0x02             |  清除 NAKTX
            |                         |                                      |
T+202       |  第1包传输完成           |  USB->EP[0].ISR & 0x010              |  IDTXIF 中断
            |                         |  USB->EP[0].ISR = 0x010              |  清除 IN 中断
            |                         |                                      |
T+203       |  主机请求剩余数据         |  <- IN TOKEN                          |
            |                         |  复制3字节到 USB_SRAM_BASE           |  剩余3字节
            |                         |  USB->EP[0].TCR = 3                  |  最后一个包
            |                         |  USB->EP[0].CSR &= ~0x02             |  清除 NAKTX
            |                         |                                      |
T+204       |  第2包传输完成           |  USB->EP[0].ISR & 0x010              |  IDTXIF 中断
            |                         |  USB->EP[0].ISR = 0x010              |  清除 IN 中断
            |                         |                                      |
T+250       |  SET_CONFIGURATION      |  <- SETUP: 00 09 01 00 00 00 00 00   |  设置配置1
            |                         |  USB->EP[0].ISR & 0x200              |  SDRXIF 中断
            |                         |  USB->EP[0].ISR = 0x200              |  清除 SETUP 中断
            |                         |                                      |
T+251       |  配置端点                |  // EP1 IN (通信端点)                 |
            |                         |  USB->EP[1].CFGR = 0x91000040        |  使能,IN,地址1,偏移0x40
            |                         |  USB->EP[1].IER = 0x010              |  IN传输中断
            |                         |                                      |
            |                         |  // EP2 OUT (数据端点)                |
            |                         |  USB->EP[2].CFGR = 0x82000080        |  使能,OUT,地址2,偏移0x80
            |                         |  USB->EP[2].IER = 0x002              |  OUT接收中断
            |                         |                                      |
            |                         |  // EP2 IN (数据端点)                 |
            |                         |  USB->EP[2].CFGR |= 0x100000C0       |  使能,IN,地址2,偏移0xC0
            |                         |  USB->EP[2].IER |= 0x010             |  IN传输中断
            |                         |                                      |
            |                         |  USB->IER |= 0x0600                  |  使能EP1,EP2中断
            |                         |                                      |
T+252       |  发送状态 ACK            |  USB->EP[0].TCR = 0                  |  零长度 IN
            |                         |  USB->EP[0].CSR &= ~0x02             |  清除 NAKTX
            |                         |                                      |
T+253       |  状态传输完成            |  USB->EP[0].ISR & 0x010              |  IDTXIF 中断
            |                         |  USB->EP[0].ISR = 0x010              |  清除 IN 中断
            |                         |                                      |
T+300       |  SET_LINE_CODING        |  <- SETUP: 21 20 00 00 00 00 07 00   |  设置串口参数
            |                         |  USB->EP[0].ISR & 0x200              |  SDRXIF 中断
            |                         |  USB->EP[0].TCR = 7                  |  准备接收7字节
            |                         |  USB->EP[0].CSR &= ~0x10             |  清除 NAKRX
            |                         |  USB->EP[0].ISR = 0x200              |  清除 SETUP 中断
            |                         |                                      |
T+301       |  接收 LINE_CODING 数据   |  <- OUT DATA: 00 C2 01 00 00 00 08   |  115200,8N1
            |                         |  USB->EP[0].ISR & 0x002              |  ODRXIF 中断
            |                         |  从 USB_SRAM_BASE 读取 7 字节         |
            |                         |  存储线路编码参数                      |
            |                         |  USB->EP[0].ISR = 0x002              |  清除 OUT 中断
            |                         |                                      |
T+302       |  发送状态 ACK            |  USB->EP[0].TCR = 0                  |  零长度 IN
            |                         |  USB->EP[0].CSR &= ~0x02             |  清除 NAKTX
            |                         |                                      |
T+350       |  SET_CONTROL_LINE_STATE |  <- SETUP: 21 22 03 00 00 00 00 00   |  DTR=1, RTS=1
            |                         |  USB->EP[0].ISR & 0x200              |  SDRXIF 中断
            |                         |  解析控制状态: wValue = 0x0003        |
            |                         |  USB->EP[0].ISR = 0x200              |  清除 SETUP 中断
            |                         |                                      |
T+351       |  发送状态 ACK            |  USB->EP[0].TCR = 0                  |  零长度 IN
            |                         |  USB->EP[0].CSR &= ~0x02             |  清除 NAKTX
            |                         |                                      |
T+400       |  枚举完成，CDC就绪        |  所有端点已配置                       |  可以进行数据传输
            |                         |                                      |
=========================================================================================================
            |                         |                                      |
            |  === 数据传输阶段 ===     |                                      |
            |                         |                                      |
T+500       |  应用层发送数据           |  cdc_send_data("Hello", 5)           |
            |                         |                                      |
T+501       |  写入 USB SRAM          |  计算地址: 0x400AA000 + 0xC0          |  EP2 IN 缓冲区
            |                         |  复制数据: "Hello" -> USB SRAM        |
            |                         |  USB->EP[2].TCR = 5                  |  设置传输计数
            |                         |  USB->EP[2].CSR &= ~0x02             |  清除 NAKTX
            |                         |                                      |
T+502       |  主机读取数据            |  <- IN TOKEN to EP2                   |
            |                         |  USB->EP[2].ISR & 0x010              |  IDTXIF 中断
            |                         |  USB->EP[2].ISR = 0x010              |  清除 IN 中断
            |                         |                                      |
T+600       |  主机发送数据            |  -> OUT DATA to EP2: "Hi"             |
            |                         |  USB->EP[2].ISR & 0x002              |  ODRXIF 中断
            |                         |  size = USB->EP[2].TCR & 0x1FF        |  读取接收字节数: 2
            |                         |  从 USB_SRAM_BASE+0x80 读取数据       |  EP2 OUT 缓冲区
            |                         |  USB->EP[2].CSR &= ~0x10             |  清除 NAKRX
            |                         |  USB->EP[2].ISR = 0x002              |  清除 OUT 中断
            |                         |  通知应用层: rx_callback("Hi", 2)     |
```

## 关键寄存器状态检查点

### 1. 复位后状态 (T+120ms)
```c
// 预期寄存器状态
USB->CSR    = 0x0400      // 只有 DPPUEN 位
USB->IER    = 0x113E      // UGIE|SOFIE|URSTIE|RSMIE|SUSPIE|EP0IE
USB->ISR    = 0x0000      // 所有中断已清除

USB->EP[0].CFGR = 0x80000000  // EPEN|地址0
USB->EP[0].IER  = 0x0212      // SDRXIE|ODRXIE|IDTXIE
USB->EP[0].ISR  = 0x0000      // 无待处理中断

USB->EP[1].CFGR = 0x00000000  // 未配置
USB->EP[2].CFGR = 0x00000000  // 未配置
```

### 2. 配置完成状态 (T+253ms)
```c
// EP0 (控制端点)
USB->EP[0].CFGR = 0x80000000  // 使能,地址0,偏移0x00,大小64字节

// EP1 (通信端点 - 中断IN)
USB->EP[1].CFGR = 0x91000040  // 使能,IN,地址1,偏移0x40,大小64字节

// EP2 (数据端点 - 批量OUT/IN)
USB->EP[2].CFGR = 0x92000080  // OUT: 使能,地址2,偏移0x80,大小64字节
                              // IN:  还需要配置IN缓冲区偏移0xC0

// 中断使能
USB->IER = 0x173E             // 基础中断 + EP0 + EP1 + EP2
```

### 3. 运行时数据传输状态
```c
// 发送前检查
assert(!(USB->EP[2].CSR & USBEPnCSR_NAKTX));  // IN 端点不在NAK状态

// 接收后检查
size_t rx_count = USB->EP[2].TCR & USBEPnTCR_TCNT;
assert(rx_count > 0 && rx_count <= 64);       // 接收字节数合理

// 中断状态检查
if (USB->EP[2].ISR & USBEPnISR_ODRXIF) {      // OUT 数据接收
    // 处理接收数据
    USB->EP[2].ISR = USBEPnISR_ODRXIF;         // 清除中断
    USB->EP[2].CSR &= ~USBEPnCSR_NAKRX;        // 准备下次接收
}

if (USB->EP[2].ISR & USBEPnISR_IDTXIF) {      // IN 数据发送完成
    USB->EP[2].ISR = USBEPnISR_IDTXIF;         // 清除中断
    // 可以发送下一个包
}
```

## 调试检查点

### 检查点 1: 连接检测 (T+10ms)
```c
void debug_connection() {
    printf("USB->CSR = 0x%04x\n", USB->CSR);
    if (USB->CSR & USBCSR_DPPUEN) {
        printf("✓ D+ 上拉已使能\n");
    } else {
        printf("✗ D+ 上拉未使能\n");
    }
}
```

### 检查点 2: 复位处理 (T+120ms)
```c
void debug_reset_complete() {
    printf("复位后状态:\n");
    printf("  USB->CSR = 0x%04x (期望: 0x0400)\n", USB->CSR);
    printf("  USB->IER = 0x%04x (期望: 0x113E)\n", USB->IER);
    printf("  EP0.CFGR = 0x%08x (期望: 0x80000000)\n", USB->EP[0].CFGR);
    printf("  EP0.IER = 0x%04x (期望: 0x0212)\n", USB->EP[0].IER);
}
```

### 检查点 3: 描述符传输 (T+150-204ms)
```c
void debug_descriptor_transfer(uint8_t *setup) {
    printf("SETUP 请求: ");
    for (int i = 0; i < 8; i++) {
        printf("%02x ", setup[i]);
    }
    printf("\n");

    uint8_t request_type = setup[0];
    uint8_t request = setup[1];
    uint16_t value = *(uint16_t*)&setup[2];
    uint16_t length = *(uint16_t*)&setup[6];

    if (request_type == 0x80 && request == 0x06) {
        printf("GET_DESCRIPTOR: ");
        switch (value >> 8) {
            case 0x01: printf("Device (长度:%d)\n", length); break;
            case 0x02: printf("Configuration (长度:%d)\n", length); break;
            case 0x03: printf("String (长度:%d)\n", length); break;
            default: printf("Unknown (0x%04x)\n", value); break;
        }
    }
}
```

### 检查点 4: 端点配置 (T+251ms)
```c
void debug_endpoint_config() {
    printf("端点配置状态:\n");
    for (int ep = 0; ep <= 2; ep++) {
        uint32_t cfgr = USB->EP[ep].CFGR;
        if (cfgr & USBEPnCFGR_EPEN) {
            uint32_t addr = (cfgr >> 24) & 0xF;
            uint32_t offset = cfgr & 0x3FF;
            uint32_t size = (cfgr >> 10) & 0x7F;
            const char* dir = (cfgr & USBEPnCFGR_EPDIR) ? "IN" : "OUT";
            const char* type = (cfgr & USBEPnCFGR_EPTYPE) ? "ISOC" : "CTRL/BULK/INTR";

            printf("  EP%d: 地址=%d, %s, %s, 偏移=0x%03x, 大小=%d字节\n",
                   ep, addr, dir, type, offset, size * 4);
        } else {
            printf("  EP%d: 禁用\n", ep);
        }
    }
}
```

### 检查点 5: 数据传输 (T+500ms+)
```c
void debug_data_transfer() {
    printf("数据传输状态:\n");

    // EP2 OUT 状态
    if (USB->EP[2].ISR & USBEPnISR_ODRXIF) {
        size_t count = USB->EP[2].TCR & USBEPnTCR_TCNT;
        printf("  EP2 OUT: 接收到 %d 字节数据\n", count);
    }

    // EP2 IN 状态
    if (USB->EP[2].CSR & USBEPnCSR_NAKTX) {
        printf("  EP2 IN: 正在发送数据...\n");
    } else {
        printf("  EP2 IN: 空闲，可以发送\n");
    }

    // USB SRAM 使用情况
    printf("  USB SRAM 分配:\n");
    printf("    EP0: 0x000-0x03F (64字节)\n");
    printf("    EP1: 0x040-0x07F (64字节)\n");
    printf("    EP2 OUT: 0x080-0x0BF (64字节)\n");
    printf("    EP2 IN:  0x0C0-0x0FF (64字节)\n");
    printf("    剩余: 0x100-0x3FF (%d字节)\n", 0x400 - 0x100);
}
```

这个详细的时序图应该能帮助你：
1. 精确定位 Embassy 实现中的问题
2. 验证每个阶段的寄存器状态
3. 确保 CDC 枚举序列的正确性
4. 调试数据传输过程

重点检查你的 Embassy 实现是否在每个时间点都达到了期望的寄存器状态，特别是端点配置和缓冲区地址分配部分。