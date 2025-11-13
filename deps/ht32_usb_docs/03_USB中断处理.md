# HT32F52352 USB中断处理详解

## 文件位置
`ChibiOS-Contrib/os/hal/ports/HT32/LLD/USBv1/hal_usb_lld.c: 184-358行`

## USB中断处理概览

```
USB_IRQHandler (USB_IRQn = 29)
    │
    ├─> SOF中断 (SOFIF)
    │     └─> _usb_isr_invoke_sof_cb()
    │
    ├─> 挂起中断 (SUSPIF)
    │     └─> _usb_suspend()
    │
    ├─> 复位中断 (URSTIF)
    │     └─> _usb_reset()
    │
    ├─> 恢复中断 (RSMIF)
    │     └─> _usb_wakeup()
    │
    ├─> EP0中断 (EP0IF)
    │     ├─> SETUP数据接收 (SDRXIF)
    │     ├─> OUT数据接收 (ODRXIF)
    │     └─> IN数据传输 (IDTXIF)
    │
    └─> EP1-7中断 (EP1IF-EP7IF)
          ├─> OUT数据接收 (ODRXIF)
          └─> IN数据传输 (IDTXIF)
```

## 中断服务程序入口

### 代码实现
```c
// hal_usb_lld.c: 184-358行
OSAL_IRQ_HANDLER(HT32_USB_IRQ_VECTOR) {
    OSAL_IRQ_PROLOGUE();
    USBDriver *usbp = &USBD1;

    // 读取中断标志 (只返回使能且待处理的中断)
    uint32_t isr = usb_get_int_flags();
    
    // 处理各类中断...
    
    OSAL_IRQ_EPILOGUE();
}
```

### 中断标志读取函数
```c
// hal_usb_lld.c: 89-91行
static uint32_t usb_get_int_flags(void) {
    // 返回使能且待处理的中断
    return USB->IER & USB->ISR;
}
```

### 中断标志清除函数
```c
// hal_usb_lld.c: 93-97行
static void usb_clear_int_flags(uint32_t flags) {
    // 中断标志通过写1清除
    // ESOFIF除外，需要正常写入
    USB->ISR = flags ^ USBISR_ESOFIF;
}
```

## 1. 帧起始(SOF)中断处理

### 触发条件
- 每个USB帧起始(每1ms)
- 仅当USBIER_SOFIE使能时触发

### 代码实现
```c
// hal_usb_lld.c: 190-195行
// 帧起始中断
if(isr & USBISR_SOFIF) {
    // 帧起始回调
    _usb_isr_invoke_sof_cb(usbp);
    
    // 清除中断标志
    usb_clear_int_flags(USBISR_SOFIF);
}
```

### 应用场景
- USB同步传输定时
- 定时器/计数器同步
- 周期性数据采样

### 使用示例
```c
// USB配置中指定SOF回调
static void sof_handler(USBDriver *usbp) {
    // 每1ms执行一次
    frame_counter++;
}

const USBConfig usbcfg = {
    NULL,                // 事件回调
    get_descriptor,      // GET_DESCRIPTOR回调
    request_handler,     // 请求处理回调
    sof_handler          // SOF回调
};
```

## 2. 挂起(SUSPEND)中断处理

### 触发条件
- USB总线空闲超过3ms
- 主机进入挂起状态

### 代码实现
```c
// hal_usb_lld.c: 197-206行
// 挂起中断
if(isr & USBISR_SUSPIF) {
    // 清除中断标志
    usb_clear_int_flags(USBISR_SUSPIF);
    
    // 挂起处理和事件回调
    _usb_suspend(usbp);
    
#if 0
    // 可选: 进入低功耗模式
    USB->CSR &= USBCSR_DPPUEN; /* POWER_ON */
    USB->CSR |= USBCSR_GENRSM;
#endif
}
```

### 挂起处理流程
```
SUSPIF中断触发
    │
    ├─> 清除中断标志
    │
    ├─> _usb_suspend(usbp)
    │       ├─ 保存当前状态
    │       ├─ 切换到SUSPENDED状态
    │       └─ 调用USB_EVENT_SUSPEND回调
    │
    └─> 可选: 进入低功耗模式
            ├─ 禁用不必要的时钟
            └─ 配置唤醒源
```

### 应用示例
```c
// USB事件回调
static void usb_event(USBDriver *usbp, usbevent_t event) {
    switch(event) {
        case USB_EVENT_SUSPEND:
            // 进入低功耗模式
            chSysLockFromISR();
            // 保存状态，禁用外设等
            chSysUnlockFromISR();
            break;
    }
}
```

## 3. USB复位(RESET)中断处理

### 触发条件
- 主机发送USB复位信号
- 总线SE0状态持续10ms以上

### 代码实现
```c
// hal_usb_lld.c: 208-213行
// 复位中断
if(isr & USBISR_URSTIF) {
    // 复位处理和事件回调
    _usb_reset(usbp);
    
    // 清除中断标志
    usb_clear_int_flags(USBISR_URSTIF);
}
```

### 复位处理流程
```
URSTIF中断触发
    │
    ├─> _usb_reset(usbp)
    │       ├─ usb_lld_reset(usbp)
    │       │       ├─ 清除CSR (保留DP上拉)
    │       │       ├─ 重置epmem_next = 8
    │       │       ├─ 初始化EP0
    │       │       └─ 重新使能中断
    │       │
    │       ├─ 重置USB驱动状态
    │       ├─ 清除设备地址
    │       ├─ 切换到READY状态
    │       └─ 调用USB_EVENT_RESET回调
    │
    └─> 清除中断标志
```

### 复位后状态
- 设备地址 = 0
- 配置值 = 0
- 所有端点禁用（EP0除外）
- EP0准备接收SETUP包

## 4. 恢复(RESUME)中断处理

### 触发条件
- 从挂起状态恢复
- 检测到总线活动或远程唤醒

### 代码实现
```c
// hal_usb_lld.c: 215-220行
// 恢复中断
if(isr & USBISR_RSMIF) {
    // 恢复/唤醒处理和回调
    _usb_wakeup(usbp);
    
    // 清除中断标志
    usb_clear_int_flags(USBISR_RSMIF);
}
```

### 恢复处理流程
```
RSMIF中断触发
    │
    ├─> _usb_wakeup(usbp)
    │       ├─ 恢复之前的状态
    │       ├─ 切换到恢复前的状态
    │       └─ 调用USB_EVENT_WAKEUP回调
    │
    └─> 清除中断标志
```

## 5. EP0中断处理

EP0是控制端点，处理USB标准请求和SETUP事务。

### 代码实现
```c
// hal_usb_lld.c: 222-292行
// EP0中断
if(isr & USBISR_EP0IF) {
    // 读取EP0中断标志
    uint32_t episr = usb_get_ep_int_flags(0);

    // 1. SETUP数据接收
    if(episr & USBEPnISR_SDRXIF) {
        // SETUP回调
        _usb_isr_invoke_setup_cb(usbp, 0);
        usb_clear_ep_int_flags(0, USBEPnISR_SDRXIF);
    }

    // 2. OUT数据接收
    if(episr & USBEPnISR_ODRXIF) {
        USBOutEndpointState *osp = usbp->epc[0]->out_state;
        size_t n = usb_packet_receive(usbp, 0);
        
        if ((n < usbp->epc[0]->out_maxsize) || (osp->rxpkts == 0)) {
            // OUT回调 (短包或完成)
            _usb_isr_invoke_out_cb(usbp, 0);
        } else {
            // 继续接收
            USB->EP[0].CSR &= USBEPnCSR_NAKRX;
        }
        usb_clear_ep_int_flags(0, USBEPnISR_ODRXIF);
    }

    // 3. IN数据传输
    if(episr & USBEPnISR_IDTXIF) {
        USBInEndpointState *isp = usbp->epc[0]->in_state;
        size_t n = isp->txlastpktlen;
        isp->txcnt += n;
        
        if (isp->txcnt < isp->txsize) {
            // 继续发送
            isp->txbuf += n;
            osalSysLockFromISR();
            usb_packet_transmit(usbp, 0, isp->txsize - isp->txcnt);
            osalSysUnlockFromISR();
        } else {
            // IN回调 (完成)
            _usb_isr_invoke_in_cb(usbp, 0);
        }
        usb_clear_ep_int_flags(0, USBEPnISR_IDTXIF);
    }

    // 清除EP0全局中断标志
    usb_clear_int_flags(USBISR_EP0IF);
}
```

### 端点中断标志读取和清除
```c
// hal_usb_lld.c: 99-106行
static uint32_t usb_get_ep_int_flags(int ep) {
    // 返回使能且待处理的端点中断
    return USB->EP[ep].IER & USB->EP[ep].ISR;
}

static void usb_clear_ep_int_flags(int ep, uint32_t flags) {
    // 端点中断标志通过写1清除
    USB->EP[ep].ISR = flags;
}
```

### 5.1 SETUP数据接收处理

#### 触发条件
- 接收到SETUP包
- 8字节SETUP数据已存储在USB SRAM

#### 处理流程
```
SDRXIF中断触发
    │
    ├─> _usb_isr_invoke_setup_cb(usbp, 0)
    │       └─> 调用用户SETUP回调
    │               └─> usb_lld_read_setup()
    │                       └─> 从USB SRAM读取8字节SETUP包
    │
    └─> 清除SDRXIF标志
```

#### SETUP包读取函数
```c
// hal_usb_lld.c: 603-614行
void usb_lld_read_setup(USBDriver *usbp, usbep_t ep, uint8_t *buf) {
    // SETUP包存储在USB SRAM起始位置
    volatile uint32_t * const EPSRAM = (void *)(USB_SRAM_BASE + 0);
    
    // 读取8字节SETUP数据 (按32位读取)
    for (size_t i = 0; i < 8; i += 4) {
        const uint32_t word = EPSRAM[i/4];
        buf[i + 0] = (word >> 0);
        buf[i + 1] = (word >> 8);
        buf[i + 2] = (word >> 16);
        buf[i + 3] = (word >> 24);
    }
    (void)usbp;
    (void)ep;
}
```

### 5.2 OUT数据接收处理

#### 触发条件
- 接收到OUT数据包
- 数据已存储在端点OUT缓冲区

#### 处理流程
```
ODRXIF中断触发
    │
    ├─> usb_packet_receive(usbp, 0)
    │       ├─ 读取TCR获取接收字节数
    │       ├─ 从USB SRAM复制数据到应用缓冲区
    │       ├─ 更新rxcnt, rxsize, rxpkts
    │       └─ 返回接收字节数
    │
    ├─> 判断是否完成
    │       ├─ 短包 (n < maxsize)  → 完成
    │       └─ rxpkts == 0        → 完成
    │
    ├─> 如果完成
    │     └─> _usb_isr_invoke_out_cb(usbp, 0)
    │
    ├─> 如果未完成
    │     └─> USB->EP[0].CSR &= USBEPnCSR_NAKRX (继续接收)
    │
    └─> 清除ODRXIF标志
```

#### OUT数据接收函数
```c
// hal_usb_lld.c: 146-171行
static size_t usb_packet_receive(USBDriver *usbp, usbep_t ep) {
    USBOutEndpointState *osp = usbp->epc[ep]->out_state;
    
    // 读取接收字节数
    // EP0: TCR[24:16]包含RX字节数
    // EP1-7: TCR[8:0]包含字节数
    const size_t n = USB->EP[ep].TCR >> ((ep == 0) ? 16 : 0);
    
    // 计算SRAM地址
    const uint32_t cfgr = USB->EP[ep].CFGR;
    volatile uint32_t *const EPSRAM = 
        (void *)(USB_SRAM_BASE + (cfgr & 0x3ff) + 
                 ((ep == 0) ? ((cfgr >> 10) & 0x7f) : 0));

    // 从USB SRAM复制数据
    for (size_t i = 0; i < n; i += 4) {
        if(!osp->rxbuf)
            break;
        const uint32_t word = EPSRAM[i/4];
        if (i + 0 < n) osp->rxbuf[i + 0] = word >> 0;
        if (i + 1 < n) osp->rxbuf[i + 1] = word >> 8;
        if (i + 2 < n) osp->rxbuf[i + 2] = word >> 16;
        if (i + 3 < n) osp->rxbuf[i + 3] = word >> 24;
    }

    // 更新状态
    osp->rxbuf += n;      // 移动缓冲区指针
    osp->rxcnt += n;      // 累加接收字节数
    osp->rxsize -= n;     // 减少剩余字节数
    osp->rxpkts--;        // 减少剩余包数
    
    return n;
}
```

### 5.3 IN数据传输处理

#### 触发条件
- IN数据包已成功发送到主机
- 需要发送下一个数据包

#### 处理流程
```
IDTXIF中断触发
    │
    ├─> 更新发送计数
    │     └─> isp->txcnt += isp->txlastpktlen
    │
    ├─> 判断是否完成
    │     └─> if (isp->txcnt < isp->txsize)
    │
    ├─> 如果未完成
    │     ├─> isp->txbuf += n (移动缓冲区指针)
    │     └─> usb_packet_transmit() (发送下一个包)
    │
    ├─> 如果完成
    │     └─> _usb_isr_invoke_in_cb(usbp, 0)
    │
    └─> 清除IDTXIF标志
```

#### IN数据发送函数
```c
// hal_usb_lld.c: 116-144行
static void usb_packet_transmit(USBDriver *usbp, usbep_t ep, size_t n) {
    const USBEndpointConfig * const epc = usbp->epc[ep];
    USBInEndpointState * const isp = epc->in_state;

    // 限制包大小不超过最大包长度
    if (n > (size_t)epc->in_maxsize)
        n = (size_t)epc->in_maxsize;

    // 检查TCR是否为0 (可以发送新包)
    if ((USB->EP[ep].TCR & 0xffffU) == 0) {
        // 计算SRAM地址
        const uint32_t cfgr = USB->EP[ep].CFGR;
        volatile uint32_t * const EPSRAM = 
            (void *)(USB_SRAM_BASE + (cfgr & 0x3ff));
        
        // 复制数据到USB SRAM (按32位写入)
        for (size_t i = 0; i < n; i += 4) {
            uint32_t word = 0;
            if (i + 0 < n) word |= (uint32_t)isp->txbuf[i+0] << 0;
            if (i + 1 < n) word |= (uint32_t)isp->txbuf[i+1] << 8;
            if (i + 2 < n) word |= (uint32_t)isp->txbuf[i+2] << 16;
            if (i + 3 < n) word |= (uint32_t)isp->txbuf[i+3] << 24;
            EPSRAM[i/4] = word;
        }

        // 设置传输字节数
        USB->EP[ep].TCR = n;
        
        // 保存本次发送的包长度
        isp->txlastpktlen = n;
        
        // 清除NAK，允许发送
        USB->EP[ep].CSR &= USBEPnCSR_NAKTX;
    }
}
```

## 6. EP1-7中断处理

普通端点(EP1-7)的处理类似EP0，但不处理SETUP包。

### 代码实现
```c
// hal_usb_lld.c: 294-356行
// EP1-7中断
uint32_t mask = USBISR_EP1IF;
for(int i = 1; i < 8; ++i) {
    // EPn中断
    if(isr & mask) {
        // 读取端点中断标志
        uint32_t episr = usb_get_ep_int_flags(i);

        // 清除端点中断标志
        usb_clear_ep_int_flags(i, episr);
        
        // 清除全局端点中断标志
        usb_clear_int_flags(mask);

        // 1. OUT数据接收
        if(episr & USBEPnISR_ODRXIF) {
            USBOutEndpointState *osp = usbp->epc[i]->out_state;
            size_t n = usb_packet_receive(usbp, i);
            
            if ((n < usbp->epc[i]->out_maxsize) || (osp->rxpkts == 0)) {
                // OUT回调 (短包或完成)
                _usb_isr_invoke_out_cb(usbp, i);
            } else {
                // 继续接收
                USB->EP[i].CSR &= USBEPnCSR_NAKRX;
            }
        }

        // 2. IN数据传输
        if(episr & USBEPnISR_IDTXIF) {
            USBInEndpointState *isp = usbp->epc[i]->in_state;
            size_t n = isp->txlastpktlen;
            isp->txcnt += n;
            
            if (isp->txcnt < isp->txsize) {
                // 继续发送
                isp->txbuf += n;
                osalSysLockFromISR();
                usb_packet_transmit(usbp, i, isp->txsize - isp->txcnt);
                osalSysUnlockFromISR();
            } else {
                // IN回调 (完成)
                _usb_isr_invoke_in_cb(usbp, i);
            }
        }
    }
    mask = mask << 1;  // 下一个端点中断掩码
}
```

### EP1-7处理流程
```
EPn中断触发 (EP1-7)
    │
    ├─> 读取端点中断标志
    │
    ├─> OUT数据接收 (ODRXIF)
    │     ├─> usb_packet_receive()
    │     ├─> 判断是否完成
    │     └─> 完成: 调用OUT回调
    │           未完成: 清除NAKRX继续接收
    │
    ├─> IN数据传输 (IDTXIF)
    │     ├─> 更新发送计数
    │     ├─> 判断是否完成
    │     └─> 完成: 调用IN回调
    │           未完成: 发送下一个包
    │
    ├─> 清除端点中断标志
    └─> 清除全局端点中断标志
```

## 7. 中断优先级和嵌套

### NVIC配置
```c
// mcuconf.h
#define HT32_USB_USB0_IRQ_PRIORITY  5  // 优先级0-15，数字越小优先级越高

// 使能USB中断
nvicEnableVector(USB_IRQn, HT32_USB_USB0_IRQ_PRIORITY);
```

### 中断嵌套注意事项
```c
// 在中断中访问临界资源需要加锁
osalSysLockFromISR();
// 临界区代码
osalSysUnlockFromISR();
```

## 8. USB数据传输状态机

### OUT传输状态机
```
IDLE (空闲)
    │
    │ usb_lld_start_out()
    ↓
RECEIVING (接收中)
    │
    │ ODRXIF中断
    ↓
    ├─> 短包? → 完成
    ├─> rxpkts == 0? → 完成
    └─> 否 → 继续接收
        │
        └─> IDLE (完成)
```

### IN传输状态机
```
IDLE (空闲)
    │
    │ usb_lld_start_in()
    ↓
SENDING (发送中)
    │
    │ IDTXIF中断
    ↓
    ├─> txcnt >= txsize? → 完成
    └─> 否 → 发送下一个包
        │
        └─> IDLE (完成)
```

## 9. 中断调试技巧

### 中断计数器
```c
// 全局变量
volatile uint32_t usb_sof_count = 0;
volatile uint32_t usb_reset_count = 0;
volatile uint32_t usb_suspend_count = 0;

// 在中断中递增
if(isr & USBISR_SOFIF) {
    usb_sof_count++;
    // ...
}
```

### 中断状态记录
```c
// 记录最后一次中断标志
volatile uint32_t last_usb_isr = 0;
volatile uint32_t last_ep_isr[8] = {0};

// 在中断中更新
OSAL_IRQ_HANDLER(HT32_USB_IRQ_VECTOR) {
    OSAL_IRQ_PROLOGUE();
    
    last_usb_isr = usb_get_int_flags();
    // ...
    
    OSAL_IRQ_EPILOGUE();
}
```

### 断点调试
```c
// 在特定条件下触发断点
if(isr & USBISR_URSTIF) {
    __asm__ volatile("bkpt 0");  // 触发断点
}
```

## 10. 常见中断问题排查

### 问题1: 中断不触发
**现象**: USB插入后无任何中断

**可能原因**:
1. 中断未使能
2. NVIC未配置
3. USB时钟未使能

**排查**:
```c
// 检查中断使能
if (!(USB->IER & USBIER_UGIE)) {
    // USB全局中断未使能
}

// 检查NVIC
if (!nvicIsEnabledVector(USB_IRQn)) {
    // NVIC未使能
}

// 检查USB时钟
if (!(CKCU->AHBCCR & CKCU_AHBCCR_USBEN)) {
    // USB时钟未使能
}
```

### 问题2: 中断风暴
**现象**: 系统卡死，中断不停触发

**可能原因**:
1. 中断标志未清除
2. 端点NAK未清除

**排查**:
```c
// 确保清除中断标志
usb_clear_int_flags(USBISR_SOFIF);

// 确保清除端点中断标志
usb_clear_ep_int_flags(ep, USBEPnISR_ODRXIF);
```

### 问题3: EP0无响应
**现象**: 枚举失败，主机无法识别设备

**可能原因**:
1. EP0中断未使能
2. SETUP回调未实现
3. 数据阶段未正确处理

**排查**:
```c
// 检查EP0中断使能
if (!(USB->IER & USBIER_EP0IE)) {
    // EP0中断未使能
}

// 检查EP0端点中断使能
if (!(USB->EP[0].IER & USBEPnIER_SDRXIE)) {
    // SETUP数据接收中断未使能
}
```

### 问题4: 数据传输不完整
**现象**: 数据只传输一部分就停止

**可能原因**:
1. 未正确处理多包传输
2. rxpkts/txcnt计算错误

**排查**:
```c
// 在OUT回调中检查
static void ep_out_cb(USBDriver *usbp, usbep_t ep) {
    USBOutEndpointState *osp = usbp->epc[ep]->out_state;
    
    // 检查是否接收完成
    if (osp->rxsize > 0) {
        // 还有数据未接收
        usb_lld_start_out(usbp, ep);
    }
}
```

## 11. 中断性能优化

### 减少中断处理时间
```c
// 在中断中只做必要的处理
OSAL_IRQ_HANDLER(HT32_USB_IRQ_VECTOR) {
    OSAL_IRQ_PROLOGUE();
    
    // 快速读取标志
    uint32_t isr = USB->IER & USB->ISR;
    
    // 快速清除标志
    USB->ISR = isr ^ USBISR_ESOFIF;
    
    // 设置事件标志，在线程中处理
    chEvtSignalI(usb_thread, isr);
    
    OSAL_IRQ_EPILOGUE();
}
```

### 批量处理中断
```c
// 使用位掩码一次性处理多个端点
uint32_t ep_mask = isr & (USBISR_EP1IF | USBISR_EP2IF | USBISR_EP3IF);
if (ep_mask) {
    // 批量处理
    for (int i = 1; i <= 3; i++) {
        if (ep_mask & (USBISR_EP0IF << i)) {
            // 处理端点i
        }
    }
}
```
