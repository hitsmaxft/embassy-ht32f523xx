# HT32F52352 USB数据传输详解

## 文件位置
`ChibiOS-Contrib/os/hal/ports/HT32/LLD/USBv1/hal_usb_lld.c`

## USB数据传输概览

```
USB数据传输类型
    │
    ├─> OUT传输 (主机→设备)
    │     ├─ usb_lld_start_out()     启动接收
    │     ├─ ODRXIF中断              数据接收
    │     ├─ usb_packet_receive()    读取数据
    │     └─ OUT回调                 传输完成
    │
    └─> IN传输 (设备→主机)
          ├─ usb_lld_start_in()      启动发送
          ├─ usb_packet_transmit()   写入数据
          ├─ IDTXIF中断              数据发送完成
          └─ IN回调                  传输完成
```

## 1. OUT传输 (接收数据)

OUT传输用于从主机接收数据到设备。

### 1.1 启动OUT传输

#### 代码实现
```c
// hal_usb_lld.c: 624-633行
void usb_lld_start_out(USBDriver *usbp, usbep_t ep) {
    USBOutEndpointState * const osp = usbp->epc[ep]->out_state;

    // 计算需要接收的包数
    if (osp->rxsize == 0)
        osp->rxpkts = 1;  // 零长度包
    else
        osp->rxpkts = (osp->rxsize + usbp->epc[ep]->out_maxsize - 1) /
            usbp->epc[ep]->out_maxsize;
    
    // 清除NAK，允许主机发送数据
    USB->EP[ep].CSR &= USBEPnCSR_NAKRX;
}
```

#### 参数说明
- **usbp**: USB驱动指针
- **ep**: 端点号 (0-7)

#### 状态准备
```c
// 应用层调用示例
USBOutEndpointState out_state;
out_state.rxbuf = rx_buffer;      // 接收缓冲区
out_state.rxsize = buffer_size;   // 期望接收的字节数
out_state.rxcnt = 0;              // 已接收字节数（初始为0）
out_state.rxpkts = 0;             // 剩余包数（由驱动计算）

// 启动接收
usbStartReceiveI(&USBD1, endpoint, rx_buffer, buffer_size);
```

#### 包数计算
```c
// 公式: rxpkts = ⌈rxsize / out_maxsize⌉
// 例子:
// rxsize = 100, out_maxsize = 64
// rxpkts = (100 + 64 - 1) / 64 = 163 / 64 = 2

// 特殊情况: rxsize = 0
// rxpkts = 1  (接收零长度包)
```

### 1.2 OUT数据接收

#### 代码实现
```c
// hal_usb_lld.c: 146-171行
static size_t usb_packet_receive(USBDriver *usbp, usbep_t ep) {
    USBOutEndpointState *osp = usbp->epc[ep]->out_state;
    
    // 1. 读取接收字节数
    const size_t n = USB->EP[ep].TCR >> ((ep == 0) ? 16 : 0);
    
    // 2. 计算USB SRAM地址
    const uint32_t cfgr = USB->EP[ep].CFGR;
    volatile uint32_t *const EPSRAM = 
        (void *)(USB_SRAM_BASE + (cfgr & 0x3ff) + 
                 ((ep == 0) ? ((cfgr >> 10) & 0x7f) : 0));

    // 3. 从USB SRAM复制数据到应用缓冲区
    for (size_t i = 0; i < n; i += 4) {
        if(!osp->rxbuf)
            break;
        const uint32_t word = EPSRAM[i/4];
        if (i + 0 < n) osp->rxbuf[i + 0] = word >> 0;
        if (i + 1 < n) osp->rxbuf[i + 1] = word >> 8;
        if (i + 2 < n) osp->rxbuf[i + 2] = word >> 16;
        if (i + 3 < n) osp->rxbuf[i + 3] = word >> 24;
    }

    // 4. 更新状态
    osp->rxbuf += n;      // 移动缓冲区指针
    osp->rxcnt += n;      // 累加接收字节数
    osp->rxsize -= n;     // 减少剩余字节数
    osp->rxpkts--;        // 减少剩余包数
    
    return n;
}
```

#### USB SRAM地址计算

##### EP0地址计算
```c
// EP0有独立的TX和RX缓冲区
// CFGR[9:0]: TX缓冲区地址
// CFGR[16:10]: TX缓冲区长度（同时也是RX缓冲区偏移）

// RX缓冲区地址 = TX缓冲区地址 + TX缓冲区长度
uint32_t tx_addr = cfgr & 0x3ff;
uint32_t tx_len = (cfgr >> 10) & 0x7f;
uint32_t rx_addr = tx_addr + tx_len;

EPSRAM = (void *)(USB_SRAM_BASE + rx_addr);
```

##### EP1-7地址计算
```c
// EP1-7使用单一缓冲区（IN或OUT）
// CFGR[9:0]: 缓冲区地址

uint32_t buf_addr = cfgr & 0x3ff;
EPSRAM = (void *)(USB_SRAM_BASE + buf_addr);
```

### 1.3 OUT中断处理

#### 代码实现
```c
// 在USB中断服务程序中
if(episr & USBEPnISR_ODRXIF) {
    USBOutEndpointState *osp = usbp->epc[ep]->out_state;
    
    // 接收数据包
    size_t n = usb_packet_receive(usbp, ep);
    
    // 判断是否完成
    // 1. 短包 (n < out_maxsize)
    // 2. 所有包已接收 (rxpkts == 0)
    if ((n < usbp->epc[ep]->out_maxsize) || (osp->rxpkts == 0)) {
        // 传输完成，调用OUT回调
        _usb_isr_invoke_out_cb(usbp, ep);
    } else {
        // 继续接收下一个包
        USB->EP[ep].CSR &= USBEPnCSR_NAKRX;
    }
    
    // 清除中断标志
    usb_clear_ep_int_flags(ep, USBEPnISR_ODRXIF);
}
```

#### 完成条件
```c
// 传输完成的两种情况:

// 1. 短包（最后一个包小于最大包长度）
if (n < out_maxsize) {
    // 传输完成
}

// 2. 所有包已接收
if (osp->rxpkts == 0) {
    // 传输完成
}

// 示例1: 接收100字节，最大包64字节
// 包1: 64字节, rxpkts = 1
// 包2: 36字节 (短包), 传输完成

// 示例2: 接收128字节，最大包64字节
// 包1: 64字节, rxpkts = 1
// 包2: 64字节, rxpkts = 0, 传输完成
```

### 1.4 OUT传输流程图

```
应用层
    │
    │ usbStartReceiveI()
    ↓
usb_lld_start_out()
    │
    ├─ 计算rxpkts
    └─ 清除NAKRX
        │
        ↓
    等待主机发送数据
        │
        │ 主机发送OUT包
        ↓
    ODRXIF中断触发
        │
        ↓
usb_packet_receive()
    │
    ├─ 读取TCR获取字节数
    ├─ 从USB SRAM复制数据
    ├─ 更新rxbuf指针
    ├─ 更新rxcnt
    ├─ 更新rxsize
    └─ rxpkts--
        │
        ↓
    判断是否完成
        │
        ├─ 短包? → 完成
        ├─ rxpkts == 0? → 完成
        └─ 否 → 清除NAKRX，继续接收
            │
            ↓
    OUT回调
        │
        └─> 应用层处理数据
```

## 2. IN传输 (发送数据)

IN传输用于从设备向主机发送数据。

### 2.1 启动IN传输

#### 代码实现
```c
// hal_usb_lld.c: 643-648行
void usb_lld_start_in(USBDriver *usbp, usbep_t ep) {
    USBInEndpointState * const isp = usbp->epc[ep]->in_state;

    // 初始化发送状态
    isp->txlastpktlen = 0;
    
    // 发送第一个数据包
    usb_packet_transmit(usbp, ep, isp->txsize);
}
```

#### 状态准备
```c
// 应用层调用示例
USBInEndpointState in_state;
in_state.txbuf = tx_buffer;       // 发送缓冲区
in_state.txsize = data_size;      // 要发送的总字节数
in_state.txcnt = 0;               // 已发送字节数（初始为0）
in_state.txlastpktlen = 0;        // 上次发送的包长度

// 启动发送
usbStartTransmitI(&USBD1, endpoint, tx_buffer, data_size);
```

### 2.2 IN数据发送

#### 代码实现
```c
// hal_usb_lld.c: 116-144行
static void usb_packet_transmit(USBDriver *usbp, usbep_t ep, size_t n) {
    const USBEndpointConfig * const epc = usbp->epc[ep];
    USBInEndpointState * const isp = epc->in_state;

    // 1. 限制包大小
    if (n > (size_t)epc->in_maxsize)
        n = (size_t)epc->in_maxsize;

    // 2. 检查TCR是否为0（可以发送）
    if ((USB->EP[ep].TCR & 0xffffU) == 0) {
        // 3. 计算USB SRAM地址
        const uint32_t cfgr = USB->EP[ep].CFGR;
        volatile uint32_t * const EPSRAM = 
            (void *)(USB_SRAM_BASE + (cfgr & 0x3ff));
        
        // 4. 复制数据到USB SRAM
        for (size_t i = 0; i < n; i += 4) {
            uint32_t word = 0;
            if (i + 0 < n) word |= (uint32_t)isp->txbuf[i+0] << 0;
            if (i + 1 < n) word |= (uint32_t)isp->txbuf[i+1] << 8;
            if (i + 2 < n) word |= (uint32_t)isp->txbuf[i+2] << 16;
            if (i + 3 < n) word |= (uint32_t)isp->txbuf[i+3] << 24;
            EPSRAM[i/4] = word;
        }

        // 5. 设置传输字节数
        USB->EP[ep].TCR = n;
        
        // 6. 保存包长度
        isp->txlastpktlen = n;
        
        // 7. 清除NAK，允许主机读取
        USB->EP[ep].CSR &= USBEPnCSR_NAKTX;
    }
}
```

#### TCR寄存器说明
```c
// TCR (Transfer Count Register) 传输计数寄存器

// EP0: 双向端点，TCR同时包含TX和RX计数
// 位[8:0]:   TX字节数（写入）
// 位[24:16]: RX字节数（读取）

// EP1-7: 单向端点，TCR只包含一个计数
// 位[8:0]: 传输字节数（IN写入，OUT读取）

// 写入TCR启动IN传输
USB->EP[ep].TCR = byte_count;

// 读取TCR获取OUT接收字节数
size_t rx_count = USB->EP[ep].TCR >> ((ep == 0) ? 16 : 0);
```

#### 数据对齐
```c
// USB SRAM按32位访问，数据需按字节组装

// 示例: 发送5字节数据 [0x12, 0x34, 0x56, 0x78, 0x9A]
// 字0 (偏移0): 0x78563412
//   字节0: 0x12
//   字节1: 0x34
//   字节2: 0x56
//   字节3: 0x78
// 字1 (偏移4): 0x0000009A
//   字节4: 0x9A
//   字节5: 未使用
//   字节6: 未使用
//   字节7: 未使用

uint32_t word0 = 0x12 | (0x34 << 8) | (0x56 << 16) | (0x78 << 24);
uint32_t word1 = 0x9A;
EPSRAM[0] = word0;
EPSRAM[1] = word1;
```

### 2.3 IN中断处理

#### 代码实现
```c
// 在USB中断服务程序中
if(episr & USBEPnISR_IDTXIF) {
    USBInEndpointState *isp = usbp->epc[ep]->in_state;
    
    // 更新已发送字节数
    size_t n = isp->txlastpktlen;
    isp->txcnt += n;
    
    // 判断是否完成
    if (isp->txcnt < isp->txsize) {
        // 未完成，发送下一个包
        isp->txbuf += n;  // 移动缓冲区指针
        osalSysLockFromISR();
        usb_packet_transmit(usbp, ep, isp->txsize - isp->txcnt);
        osalSysUnlockFromISR();
    } else {
        // 传输完成，调用IN回调
        _usb_isr_invoke_in_cb(usbp, ep);
    }
    
    // 清除中断标志
    usb_clear_ep_int_flags(ep, USBEPnISR_IDTXIF);
}
```

#### 完成条件
```c
// 传输完成条件:
if (isp->txcnt >= isp->txsize) {
    // 所有数据已发送
}

// 示例1: 发送100字节，最大包64字节
// 包1: 64字节, txcnt = 64
// 包2: 36字节, txcnt = 100, 传输完成

// 示例2: 发送128字节，最大包64字节
// 包1: 64字节, txcnt = 64
// 包2: 64字节, txcnt = 128, 传输完成
```

### 2.4 零长度包(ZLP)处理

#### 何时需要ZLP
```c
// 当传输大小是最大包长度的整数倍时，需要发送ZLP
// 告诉主机传输已完成

// 示例: 发送64字节，最大包64字节
// 包1: 64字节
// 包2: 0字节 (ZLP) ← 表示传输完成

// 如果不发送ZLP，主机会继续等待数据
```

#### ChibiOS自动处理ZLP
```c
// ChibiOS USB栈在_usb_ep0in中自动处理ZLP
// 应用层通常不需要手动发送ZLP

// 手动发送ZLP示例（如果需要）
USBInEndpointState in_state;
in_state.txbuf = NULL;         // 无数据
in_state.txsize = 0;           // 零长度
in_state.txcnt = 0;
usbStartTransmitI(&USBD1, ep, NULL, 0);
```

### 2.5 IN传输流程图

```
应用层
    │
    │ usbStartTransmitI()
    ↓
usb_lld_start_in()
    │
    └─ usb_packet_transmit()
        │
        ├─ 限制包大小
        ├─ 复制数据到USB SRAM
        ├─ 设置TCR
        └─ 清除NAKTX
            │
            ↓
        等待主机读取
            │
            │ 主机发送IN令牌
            ↓
        硬件发送数据
            │
            ↓
        IDTXIF中断触发
            │
            ├─ txcnt += txlastpktlen
            ↓
        判断是否完成
            │
            ├─ txcnt >= txsize? → 完成
            └─ 否 → 发送下一个包
                │
                ↓
            IN回调
                │
                └─> 应用层通知
```

## 3. 端点状态管理

### 3.1 OUT端点状态

#### 状态结构
```c
// hal_usb_lld.h: 110-131行
typedef struct {
    size_t rxsize;         // 请求接收的总字节数
    size_t rxcnt;          // 已接收的字节数
    uint8_t *rxbuf;        // 接收缓冲区指针
#if (USB_USE_WAIT == TRUE)
    thread_reference_t thread;  // 等待线程
#endif
    uint16_t rxpkts;       // 剩余待接收的包数
} USBOutEndpointState;
```

#### 状态更新
```c
// 初始化
osp->rxsize = total_size;    // 总字节数
osp->rxcnt = 0;              // 已接收0字节
osp->rxbuf = buffer;         // 缓冲区地址
osp->rxpkts = (rxsize + maxsize - 1) / maxsize;  // 计算包数

// 每次接收后更新
osp->rxbuf += n;             // 移动指针
osp->rxcnt += n;             // 累加计数
osp->rxsize -= n;            // 减少剩余
osp->rxpkts--;               // 减少包数

// 检查完成
if ((n < maxsize) || (osp->rxpkts == 0)) {
    // 传输完成
}
```

### 3.2 IN端点状态

#### 状态结构
```c
// hal_usb_lld.h: 84-105行
typedef struct {
    size_t txsize;         // 请求发送的总字节数
    size_t txcnt;          // 已发送的字节数
    const uint8_t *txbuf;  // 发送缓冲区指针
#if (USB_USE_WAIT == TRUE)
    thread_reference_t thread;  // 等待线程
#endif
    uint16_t txlastpktlen; // 上次发送的包长度
} USBInEndpointState;
```

#### 状态更新
```c
// 初始化
isp->txsize = total_size;    // 总字节数
isp->txcnt = 0;              // 已发送0字节
isp->txbuf = buffer;         // 缓冲区地址
isp->txlastpktlen = 0;       // 包长度

// 发送时更新
isp->txlastpktlen = n;       // 保存包长度

// 发送完成后更新
isp->txcnt += n;             // 累加计数
isp->txbuf += n;             // 移动指针

// 检查完成
if (isp->txcnt >= isp->txsize) {
    // 传输完成
}
```

## 4. 端点STALL和NAK控制

### 4.1 STALL状态

#### 设置STALL
```c
// hal_usb_lld.c: 658-663行
void usb_lld_stall_out(USBDriver *usbp, usbep_t ep) {
    (void)usbp;
    USB->EP[ep].CSR = USBEPnCSR_STLRX;
}

// hal_usb_lld.c: 673-678行
void usb_lld_stall_in(USBDriver *usbp, usbep_t ep) {
    (void)usbp;
    USB->EP[ep].CSR = USBEPnCSR_STLTX;
}
```

#### 清除STALL
```c
// hal_usb_lld.c: 688-693行
void usb_lld_clear_out(USBDriver *usbp, usbep_t ep) {
    (void)usbp;
    USB->EP[ep].CSR &= USBEPnCSR_STLRX;
}

// hal_usb_lld.c: 703-708行
void usb_lld_clear_in(USBDriver *usbp, usbep_t ep) {
    (void)usbp;
    USB->EP[ep].CSR &= USBEPnCSR_STLTX;
}
```

#### STALL应用场景
```c
// 1. 不支持的请求
if (unsupported_request) {
    usbStallReceiveI(&USBD1, ep);
    usbStallTransmitI(&USBD1, ep);
}

// 2. 协议错误
if (protocol_error) {
    usb_lld_stall_out(&USBD1, ep);
}

// 3. 端点Halt特性
// USB规范要求的Halt功能
USB_ENDPOINT_HALT 特性
```

### 4.2 NAK状态

#### NAK自动管理
```c
// OUT端点: 清除NAKRX允许接收
USB->EP[ep].CSR &= USBEPnCSR_NAKRX;

// IN端点: 清除NAKTX允许发送
USB->EP[ep].CSR &= USBEPnCSR_NAKTX;

// NAK状态由硬件自动设置:
// - OUT: 数据接收后自动设置NAKRX
// - IN: 数据发送后自动设置NAKTX
```

#### NAK用途
```c
// NAK告诉主机"暂时不能处理"
// 主机会稍后重试

// OUT端点: 缓冲区满时NAK
if (buffer_full) {
    // 不清除NAKRX，主机会重试
}

// IN端点: 数据未准备好时NAK
if (!data_ready) {
    // 不清除NAKTX，主机会重试
}
```

## 5. 端点状态查询

### 5.1 查询OUT端点状态

```c
// hal_usb_lld.c: 551-562行
usbepstatus_t usb_lld_get_status_out(USBDriver *usbp, usbep_t ep) {
    (void)usbp;

    // 检查端点号
    if (ep > USB_MAX_ENDPOINTS)
        return EP_STATUS_DISABLED;
    
    // 检查端点是否使能
    if ((USB->EP[ep].CFGR & USBEPnCFGR_EPEN) == 0)
        return EP_STATUS_DISABLED;
    
    // 检查STALL状态
    if ((USB->EP[ep].CSR & USBEPnCSR_STLRX) != 0)
        return EP_STATUS_STALLED;
    
    return EP_STATUS_ACTIVE;
}
```

### 5.2 查询IN端点状态

```c
// hal_usb_lld.c: 576-587行
usbepstatus_t usb_lld_get_status_in(USBDriver *usbp, usbep_t ep) {
    (void)usbp;

    // 检查端点号
    if (ep > USB_MAX_ENDPOINTS)
        return EP_STATUS_DISABLED;
    
    // 检查端点是否使能
    if ((USB->EP[ep].CFGR & USBEPnCFGR_EPEN) == 0)
        return EP_STATUS_DISABLED;
    
    // 检查STALL状态
    if ((USB->EP[ep].CSR & USBEPnCSR_STLRX) != 0)
        return EP_STATUS_STALLED;
    
    return EP_STATUS_ACTIVE;
}
```

### 5.3 状态枚举
```c
typedef enum {
    EP_STATUS_DISABLED = 0,  // 端点禁用
    EP_STATUS_STALLED = 1,   // 端点STALL
    EP_STATUS_ACTIVE = 2     // 端点活动
} usbepstatus_t;
```

## 6. 获取传输大小

```c
// hal_usb_lld.h: 337-338行
#define usb_lld_get_transaction_size(usbp, ep) \
    ((usbp)->epc[ep]->out_state->rxcnt)

// 使用示例
size_t received = usb_lld_get_transaction_size(&USBD1, ep);
```

## 7. 应用层接口

### 7.1 启动接收
```c
// 阻塞式接收
size_t n = usbReceive(&USBD1, ep, buffer, size);

// 非阻塞式接收
usbStartReceiveI(&USBD1, ep, buffer, size);
// ... 在OUT回调中处理
```

### 7.2 启动发送
```c
// 阻塞式发送
usbTransmit(&USBD1, ep, buffer, size);

// 非阻塞式发送
usbStartTransmitI(&USBD1, ep, buffer, size);
// ... 在IN回调中处理
```

## 8. 多包传输示例

### 8.1 接收大文件
```c
#define FILE_SIZE   10240  // 10KB
#define MAX_PKT     64     // 64字节最大包

uint8_t file_buffer[FILE_SIZE];
size_t file_received = 0;

// OUT端点回调
static void data_received(USBDriver *usbp, usbep_t ep) {
    size_t n = usb_lld_get_transaction_size(usbp, ep);
    file_received += n;
    
    if (file_received < FILE_SIZE) {
        // 继续接收
        usbStartReceiveI(usbp, ep, 
                        file_buffer + file_received,
                        FILE_SIZE - file_received);
    } else {
        // 接收完成
        process_file(file_buffer, file_received);
    }
}

// 启动接收
usbStartReceiveI(&USBD1, DATA_EP, file_buffer, FILE_SIZE);
```

### 8.2 发送大文件
```c
#define FILE_SIZE   10240  // 10KB
uint8_t file_buffer[FILE_SIZE];
size_t file_sent = 0;

// IN端点回调
static void data_sent(USBDriver *usbp, usbep_t ep) {
    file_sent += usbp->epc[ep]->in_state->txlastpktlen;
    
    if (file_sent < FILE_SIZE) {
        // 继续发送
        size_t remaining = FILE_SIZE - file_sent;
        usbStartTransmitI(usbp, ep,
                         file_buffer + file_sent,
                         remaining);
    } else {
        // 发送完成
        file_sent = 0;  // 重置计数器
    }
}

// 启动发送
usbStartTransmitI(&USBD1, DATA_EP, file_buffer, FILE_SIZE);
```

## 9. 常见问题和优化

### 问题1: 数据丢失
**现象**: 接收到的数据不完整

**原因**: 缓冲区被覆盖，未及时处理

**解决**:
```c
// 使用双缓冲
uint8_t buffer0[SIZE];
uint8_t buffer1[SIZE];
uint8_t current = 0;

static void out_cb(USBDriver *usbp, usbep_t ep) {
    // 处理当前缓冲区
    process_data(current ? buffer1 : buffer0);
    
    // 切换缓冲区
    current = !current;
    
    // 启动下一次接收
    usbStartReceiveI(usbp, ep,
                     current ? buffer1 : buffer0,
                     SIZE);
}
```

### 问题2: 发送卡死
**现象**: IN传输无响应

**原因**: TCR未清零就尝试发送

**解决**:
```c
// 检查TCR
if ((USB->EP[ep].TCR & 0xffffU) == 0) {
    // 可以发送
    usb_packet_transmit(usbp, ep, size);
} else {
    // 等待上一次传输完成
}
```

### 问题3: 传输不完整
**现象**: 只传输了部分数据

**原因**: 多包传输未正确处理

**解决**:
```c
// 确保在回调中继续传输
static void in_cb(USBDriver *usbp, usbep_t ep) {
    if (remaining_data > 0) {
        usbStartTransmitI(usbp, ep, next_data, remaining_data);
    }
}
```

### 优化1: 使用DMA (如果硬件支持)
```c
// HT32F52352的USB不直接支持DMA
// 但可以使用PDMA在后台复制数据

// 在OUT回调中启动DMA
pdmaChannelSetup(channel, EPSRAM, app_buffer, size);
```

### 优化2: 批量传输
```c
// 累积小数据包，一次性发送
#define BATCH_SIZE  512
uint8_t batch_buffer[BATCH_SIZE];
size_t batch_count = 0;

void add_data(uint8_t *data, size_t size) {
    if (batch_count + size > BATCH_SIZE) {
        // 发送当前批次
        usbStartTransmitI(&USBD1, ep, batch_buffer, batch_count);
        batch_count = 0;
    }
    memcpy(batch_buffer + batch_count, data, size);
    batch_count += size;
}
```
