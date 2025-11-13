# HT32F52352 USB寄存器定义速查

## 文件位置
`ChibiOS-Contrib/os/common/ext/CMSIS/HT32/HT32F523xx/ht32f523x2_reg.h`

## USB基础地址定义

```c
#define USB_BASE                ((uint32_t)0x400A8000)
#define USB_SRAM_BASE           ((uint32_t)0x400AA000)
#define USB                     ((USB_TypeDef *)    USB_BASE)
```

- **USB_BASE**: USB外设基地址 `0x400A8000`
- **USB_SRAM_BASE**: USB SRAM基地址 `0x400AA000` (用于端点缓冲区)
- **USB**: USB外设寄存器指针

## USB寄存器结构体定义

```c
typedef struct {
  __IO uint32_t CSR;            // 0x000  USB控制和状态寄存器
  __IO uint32_t IER;            // 0x004  USB中断使能寄存器
  __IO uint32_t ISR;            // 0x008  USB中断状态寄存器
  __IO uint32_t FCR;            // 0x00C  USB帧计数寄存器
  __IO uint32_t DEVAR;          // 0x010  USB设备地址寄存器
  struct {
    __IO uint32_t CSR;          // 0x014  USB端点n控制和状态寄存器
    __IO uint32_t IER;          // 0x018  USB端点n中断使能寄存器
    __IO uint32_t ISR;          // 0x01C  USB端点n中断状态寄存器
    __IO uint32_t TCR;          // 0x020  USB端点n传输计数寄存器
    __IO uint32_t CFGR;         // 0x024  USB端点n配置寄存器
  } EP[8];                      // 8个端点配置
} USB_TypeDef;
```

## USB控制和状态寄存器 (USB->CSR)

```c
#define USBCSR_FRES         (0x002)    // 强制USB复位控制
#define USBCSR_PDWN         (0x004)    // 掉电模式控制
#define USBCSR_LPMODE       (0x008)    // 低功耗模式控制
#define USBCSR_GENRSM       (0x020)    // 恢复请求生成控制
#define USBCSR_RXDP         (0x040)    // 接收DP线状态
#define USBCSR_RXDM         (0x080)    // 接收DM线状态
#define USBCSR_ADRSET       (0x100)    // 设备地址设置控制
#define USBCSR_SRAMRSTC     (0x200)    // USB SRAM复位条件
#define USBCSR_DPPUEN       (0x400)    // DP上拉使能
#define USBCSR_DPWKEN       (0x800)    // DP唤醒使能
```

### 关键操作
```c
// 连接USB设备 (启用DP上拉)
USB->CSR |= USBCSR_DPPUEN;

// 断开USB设备 (禁用DP上拉)
USB->CSR &= ~USBCSR_DPPUEN;

// 生成恢复信号
USB->CSR |= USBCSR_GENRSM;

// 设置设备地址
USB->CSR |= USBCSR_ADRSET;
USB->DEVAR = address & 0x7f;
```

## USB中断使能寄存器 (USB->IER)

```c
#define USBIER_UGIE         (0x0001)   // USB全局中断使能
#define USBIER_SOFIE        (0x0002)   // 帧起始中断使能
#define USBIER_URSTIE       (0x0004)   // USB复位中断使能
#define USBIER_RSMIE        (0x0008)   // 恢复中断使能
#define USBIER_SUSPIE       (0x0010)   // 挂起中断使能
#define USBIER_ESOFIE       (0x0020)   // 期望帧起始使能
#define USBIER_EP0IE        (0x0100)   // 端点0中断使能
#define USBIER_EP1IE        (0x0200)   // 端点1中断使能
#define USBIER_EP2IE        (0x0400)   // 端点2中断使能
#define USBIER_EP3IE        (0x0800)   // 端点3中断使能
#define USBIER_EP4IE        (0x1000)   // 端点4中断使能
#define USBIER_EP5IE        (0x2000)   // 端点5中断使能
#define USBIER_EP6IE        (0x4000)   // 端点6中断使能
#define USBIER_EP7IE        (0x8000)   // 端点7中断使能
```

## USB中断状态寄存器 (USB->ISR)

```c
#define USBISR_SOFIF        (0x0002)   // 帧起始中断标志
#define USBISR_URSTIF       (0x0004)   // USB复位中断标志
#define USBISR_RSMIF        (0x0008)   // 恢复中断标志
#define USBISR_SUSPIF       (0x0010)   // 挂起中断标志
#define USBISR_ESOFIF       (0x0020)   // 期望帧起始中断
#define USBISR_EP0IF        (1U << 8)  // 端点0中断标志
#define USBISR_EP1IF        (1U << 9)  // 端点1中断标志
// ... EP2-EP7类似
#define USBISR_EPnIF        (0xFF00)   // 端点中断掩码
```

### 中断标志清除
```c
// 中断标志通过写1清除（ESOFIF除外）
// ESOFIF需要正常写入
USB->ISR = flags ^ USBISR_ESOFIF;
```

## USB帧计数寄存器 (USB->FCR)

```c
#define USBFCR_FRNUM        (0x7FF)        // 帧号 (11位)
#define USBFCR_SOFLCK       (1U << 16)     // 帧起始锁定标志
#define USBFCR_LSOF         (0x3U << 17)   // 丢失帧起始数量
```

```c
// 读取当前帧号
uint16_t frame_number = USB->FCR & USBFCR_FRNUM;
```

## USB端点控制和状态寄存器 (USB->EP[n].CSR)

```c
#define USBEPnCSR_DTGTX     (0x01)  // 数据翻转状态(IN传输)
#define USBEPnCSR_NAKTX     (0x02)  // NAK状态(IN传输)
#define USBEPnCSR_STLTX     (0x04)  // STALL状态(IN传输)
#define USBEPnCSR_DTGRX     (0x08)  // 数据翻转状态(OUT传输)
#define USBEPnCSR_NAKRX     (0x10)  // NAK状态(OUT传输)
#define USBEPnCSR_STLRX     (0x20)  // STALL状态(OUT传输)
```

### 端点操作
```c
// 设置IN端点STALL
USB->EP[ep].CSR = USBEPnCSR_STLTX;

// 设置OUT端点STALL
USB->EP[ep].CSR = USBEPnCSR_STLRX;

// 清除OUT端点STALL
USB->EP[ep].CSR &= USBEPnCSR_STLRX;

// 清除IN端点STALL
USB->EP[ep].CSR &= USBEPnCSR_STLTX;

// 清除OUT端点NAK (允许接收)
USB->EP[ep].CSR &= USBEPnCSR_NAKRX;

// 清除IN端点NAK (允许发送)
USB->EP[ep].CSR &= USBEPnCSR_NAKTX;
```

## USB端点中断使能寄存器 (USB->EP[n].IER)

```c
#define USBEPnIER_OTRXIE    (0x001)  // OUT令牌接收中断使能
#define USBEPnIER_ODRXIE    (0x002)  // OUT数据接收中断使能
#define USBEPnIER_ODOVIE    (0x004)  // OUT数据缓冲区溢出中断使能
#define USBEPnIER_ITRXIE    (0x008)  // IN令牌接收中断使能
#define USBEPnIER_IDTXIE    (0x010)  // IN数据传输完成中断使能
#define USBEPnIER_NAKIE     (0x020)  // NAK传输中断使能
#define USBEPnIER_STLIE     (0x040)  // STALL传输中断使能
#define USBEPnIER_UERIE     (0x080)  // USB错误中断使能
#define USBEPnIER_STRXIE    (0x100)  // SETUP令牌接收中断使能
#define USBEPnIER_SDRXIE    (0x200)  // SETUP数据接收中断使能
#define USBEPnIER_SDERIE    (0x400)  // SETUP数据错误中断使能
#define USBEPnIER_ZLRXIE    (0x800)  // 零长度数据接收中断使能
```

## USB端点中断状态寄存器 (USB->EP[n].ISR)

```c
#define USBEPnISR_OTRXIF    (0x001)  // OUT令牌接收中断标志
#define USBEPnISR_ODRXIF    (0x002)  // OUT数据接收中断标志
#define USBEPnISR_ODOVIF    (0x004)  // OUT数据缓冲区溢出中断标志
#define USBEPnISR_ITRXIF    (0x008)  // IN令牌接收中断标志
#define USBEPnISR_IDTXIF    (0x010)  // IN数据传输完成中断标志
#define USBEPnISR_NAKIF     (0x020)  // NAK传输中断标志
#define USBEPnISR_STLIF     (0x040)  // STALL传输中断标志
#define USBEPnISR_UERIF     (0x080)  // USB错误中断标志
#define USBEPnISR_STRXIF    (0x100)  // SETUP令牌接收中断标志
#define USBEPnISR_SDRXIF    (0x200)  // SETUP数据接收中断标志
#define USBEPnISR_SDERIF    (0x400)  // SETUP数据错误中断标志
#define USBEPnISR_ZLRXIF    (0x800)  // 零长度数据接收中断标志
```

```c
// 端点中断标志通过写1清除
USB->EP[ep].ISR = flags;
```

## USB端点传输计数寄存器 (USB->EP[n].TCR)

```c
#define USBEPnTCR_TCNT      (0x1FF)  // 传输字节计数 (9位，最大512字节)
```

### TCR寄存器布局
- **EP0**: 
  - 位[8:0]: TX字节计数
  - 位[24:16]: RX字节计数
- **EP1-7**: 
  - 位[8:0]: 传输字节计数

```c
// 设置IN端点传输字节数
USB->EP[ep].TCR = byte_count;

// 读取OUT端点接收字节数
size_t rx_count = USB->EP[ep].TCR >> ((ep == 0) ? 16 : 0);
```

## USB端点配置寄存器 (USB->EP[n].CFGR)

```c
#define USBEPnCFGR_EPEN     (1U << 31)     // 端点使能
#define USBEPnCFGR_EPTYPE   (1U << 29)     // 传输类型 (0=控制/批量/中断, 1=同步)
#define USBEPnCFGR_EPDIR    (1U << 28)     // 传输方向 (0=OUT, 1=IN)
#define USBEPnCFGR_EPADR    (0xFU << 24)   // 端点地址 (4位)
#define USBEPnCFGR_EPLEN    (0x7FU << 10)  // 缓冲区长度 (7位)
#define USBEPnCFGR_EPBUFA   (0x3FF)        // 端点缓冲区地址 (10位)
```

### 端点配置示例
```c
// 配置IN端点
uint32_t cfgr = USBEPnCFGR_EPEN |           // 使能端点
                ((uint32_t)ep << 24) |      // 端点地址
                (buffer_offset << 0) |       // 缓冲区偏移
                (buffer_size << 10) |        // 缓冲区大小
                USBEPnCFGR_EPDIR;           // IN方向

// 配置批量端点 (BULK)
cfgr |= 0;  // EPTYPE = 0

// 配置同步端点 (ISOCHRONOUS)
cfgr |= USBEPnCFGR_EPTYPE;

USB->EP[ep].CFGR = cfgr;
```

## 时钟配置寄存器 (CKCU)

```c
// USB时钟使能
#define CKCU_AHBCCR_USBEN   (1U << 10)  // USB时钟使能位

// USB预分频器配置
#define CKCU_GCFGR_USBPRE_MASK  (3U << 22)

// 使能USB时钟
CKCU->GCFGR = (CKCU->GCFGR & ~CKCU_GCFGR_USBPRE_MASK) | 
              ((HT32_USB_PRESCALER - 1) << 22);
CKCU->AHBCCR |= CKCU_AHBCCR_USBEN;
```

## 复位控制寄存器 (RSTCU)

```c
// USB复位控制
#define RSTCU_AHBPRSTR_USBRST  (1U << 5)

// 复位USB外设
RSTCU->AHBPRSTR = RSTCU_AHBPRSTR_USBRST;
```

## USB中断号定义

```c
typedef enum IRQn {
    // ...
    USB_IRQn = 29,  // USB中断号
    // ...
} IRQn_Type;
```

```c
// 使能USB中断
nvicEnableVector(USB_IRQn, HT32_USB_USB0_IRQ_PRIORITY);

// 禁用USB中断
nvicDisableVector(USB_IRQn);
```
