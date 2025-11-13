# HT32F52352 USB初始化流程详解

## 文件位置
- 驱动源文件: `ChibiOS-Contrib/os/hal/ports/HT32/LLD/USBv1/hal_usb_lld.c`
- 驱动头文件: `ChibiOS-Contrib/os/hal/ports/HT32/LLD/USBv1/hal_usb_lld.h`

## 初始化流程概览

```
1. usb_lld_init()          // 低级USB驱动初始化
   ├─ 配置USB时钟
   └─ 使能USB时钟

2. usb_lld_start()         // 启动USB外设
   ├─ 使能USB中断
   ├─ 配置USB控制寄存器
   └─ 使能USB事件中断

3. usb_lld_reset()         // USB复位
   ├─ 清除CSR (保留DP上拉)
   ├─ 初始化端点内存分配器
   └─ 初始化EP0

4. usbStart()              // 启动USB驱动 (应用层)
5. usbConnectBus()         // 连接USB总线
```

## 1. 低级USB驱动初始化 (usb_lld_init)

### 代码位置
`hal_usb_lld.c: 371-381行`

### 功能
初始化USB外设，配置时钟和使能USB控制器。

### 代码实现
```c
void usb_lld_init(void) {
#if HT32_USB_USE_USB0 == TRUE
    /* 驱动初始化 */
    usbObjectInit(&USBD1);

    // USB预分频器配置
    // 公式: USB_CLK = PLL_CLK / HT32_USB_PRESCALER
    // 目标: USB_CLK = 48MHz
    CKCU->GCFGR = (CKCU->GCFGR & ~CKCU_GCFGR_USBPRE_MASK) | 
                  ((HT32_USB_PRESCALER - 1) << 22);
    
    // 使能USB时钟
    CKCU->AHBCCR |= CKCU_AHBCCR_USBEN;
#endif
}
```

### 时钟配置示例 (mcuconf.h)
```c
#define HT32_CK_HSE_FREQUENCY   8000000UL    // 8 MHz外部晶振
#define HT32_PLL_FBDIV          18           // PLL倍频: 8MHz * 18 = 144MHz
#define HT32_AHB_PRESCALER      2            // AHB分频: 144MHz / 2 = 72MHz
#define HT32_USB_PRESCALER      3            // USB分频: 144MHz / 3 = 48MHz
```

### 关键参数
- **HT32_USB_PRESCALER**: USB时钟预分频器 (必须配置为48MHz)
- **CKCU_GCFGR_USBPRE_MASK**: USB预分频器掩码 (位[23:22])
- **CKCU_AHBCCR_USBEN**: USB时钟使能位

### 注意事项
1. USB时钟必须为48MHz (±0.25%)
2. 先配置预分频器，再使能时钟
3. 确保PLL已正确配置并稳定

## 2. USB外设启动 (usb_lld_start)

### 代码位置
`hal_usb_lld.c: 390-409行`

### 功能
配置并启动USB外设，使能中断和USB控制寄存器。

### 代码实现
```c
void usb_lld_start(USBDriver *usbp) {
    if (usbp->state == USB_STOP) {
        /* 使能外设 */
#if HT32_USB_USE_USB0 == TRUE
        if (&USBD1 == usbp) {
            // 使能USB中断
            nvicEnableVector(USB_IRQn, HT32_USB_USB0_IRQ_PRIORITY);
            
            /* USBD上电 */
            // CSR配置:
            // - DPWKEN: DP唤醒使能
            // - DPPUEN: DP上拉使能
            // - LPMODE: 低功耗模式
            // - PDWN: 掉电模式
            USB->CSR = USBCSR_DPWKEN | USBCSR_DPPUEN | 
                       USBCSR_LPMODE | USBCSR_PDWN;
            
            // 清除所有中断标志
            USB->ISR = ~0U;
            
            // 禁用DP唤醒
            USB->CSR &= ~USBCSR_DPWKEN;
            
            // 使能USB中断
            USB->IER = USBIER_UGIE |      // USB全局中断使能
                       USBIER_SOFIE |     // 帧起始中断使能
                       USBIER_URSTIE |    // USB复位中断使能
                       USBIER_RSMIE |     // 恢复中断使能
                       USBIER_SUSPIE |    // 挂起中断使能
                       USBIER_EP0IE;      // 端点0中断使能
        }
#endif
    }
    /* 配置外设 */
}
```

### USB控制寄存器(CSR)初始化步骤

#### 步骤1: 上电配置
```c
USB->CSR = USBCSR_DPWKEN |   // DP唤醒使能 (用于挂起恢复)
           USBCSR_DPPUEN |   // DP上拉使能 (连接到主机)
           USBCSR_LPMODE |   // 低功耗模式
           USBCSR_PDWN;      // 掉电模式
```

#### 步骤2: 清除中断标志
```c
USB->ISR = ~0U;  // 清除所有挂起的中断
```

#### 步骤3: 禁用DP唤醒
```c
USB->CSR &= ~USBCSR_DPWKEN;  // 正常运行时禁用唤醒
```

#### 步骤4: 使能中断
```c
USB->IER = USBIER_UGIE |      // 全局中断使能
           USBIER_SOFIE |     // 帧起始中断
           USBIER_URSTIE |    // USB复位中断
           USBIER_RSMIE |     // 恢复中断
           USBIER_SUSPIE |    // 挂起中断
           USBIER_EP0IE;      // 端点0中断
```

### 中断优先级配置
```c
// mcuconf.h
#define HT32_USB_USB0_IRQ_PRIORITY  5  // USB中断优先级 (0-15)

// 使能NVIC中断
nvicEnableVector(USB_IRQn, HT32_USB_USB0_IRQ_PRIORITY);
```

## 3. USB复位处理 (usb_lld_reset)

### 代码位置
`hal_usb_lld.c: 441-456行`

### 功能
处理USB总线复位事件，重新初始化USB外设和端点0。

### 代码实现
```c
void usb_lld_reset(USBDriver *usbp) {
    // USB复位
    // 清除CSR，保留DP上拉
    USB->CSR &= USBCSR_DPPUEN;

    /* 复位后初始化 */
    // 初始化端点内存分配器
    // EP0保留前8字节用于SETUP包
    usbp->epmem_next = 8;

    /* EP0初始化 */
    usbp->epc[0] = &ep0config;
    usb_lld_init_endpoint(usbp, 0);

    // 重新使能USB中断
    USB->IER = USBIER_UGIE |      // USB全局中断使能
               USBIER_SOFIE |     // 帧起始中断使能
               USBIER_URSTIE |    // USB复位中断使能
               USBIER_RSMIE |     // 恢复中断使能
               USBIER_SUSPIE |    // 挂起中断使能
               USBIER_EP0IE;      // 端点0中断使能
}
```

### EP0配置结构
```c
// EP0初始化配置 (hal_usb_lld.c: 70-79行)
static const USBEndpointConfig ep0config = {
    USB_EP_MODE_TYPE_CTRL,   // 端点类型: 控制端点
    _usb_ep0setup,           // SETUP包回调
    _usb_ep0in,              // IN传输回调
    _usb_ep0out,             // OUT传输回调
    0x40,                    // IN最大包大小: 64字节
    0x40,                    // OUT最大包大小: 64字节
    &ep0_state.in,           // IN端点状态
    &ep0_state.out           // OUT端点状态
};
```

### USB SRAM内存分配

HT32F52352的USB SRAM大小为1024字节 (0x400)

```c
// 内存分配策略
usbp->epmem_next = 8;  // 起始地址，前8字节保留给EP0 SETUP

// 内存对齐要求: 4字节对齐
#define roundup2(x, m) (((x) + (m) - 1) & ~((m) - 1))

// 分配端点缓冲区
static size_t usb_epmem_alloc(USBDriver *usbp, size_t size) {
    const size_t epmo = usbp->epmem_next;
    
    // 4字节对齐
    usbp->epmem_next = roundup2(epmo + size, 4);
    
    // 检查是否超出SRAM大小
    osalDbgAssert(usbp->epmem_next <= 0x400, "EPSRAM exhausted");
    
    return epmo;  // 返回分配的偏移地址
}
```

### USB SRAM布局示例
```
地址偏移    大小    用途
0x000      8B     EP0 SETUP包缓冲区
0x008      64B    EP0 TX缓冲区
0x048      64B    EP0 RX缓冲区
0x088      64B    EP1 缓冲区
...               其他端点
```

## 4. 端点初始化 (usb_lld_init_endpoint)

### 代码位置
`hal_usb_lld.c: 478-520行`

### 功能
配置单个USB端点，包括类型、方向、缓冲区和中断。

### 代码实现
```c
void usb_lld_init_endpoint(USBDriver *usbp, usbep_t ep) {
    if(ep > USB_MAX_ENDPOINTS)
        return;

    const USBEndpointConfig *epcp = usbp->epc[ep];
    uint32_t cfgr = USBEPnCFGR_EPEN |        // 使能端点
                    ((uint32_t)ep << 24);    // 端点地址
    size_t epmo;

    // 配置端点类型
    switch(epcp->ep_mode & USB_EP_MODE_TYPE) {
        case USB_EP_MODE_TYPE_CTRL:    // 控制端点
            break;
        case USB_EP_MODE_TYPE_ISOC:    // 同步端点
            cfgr |= USBEPnCFGR_EPTYPE;
            break;
        case USB_EP_MODE_TYPE_BULK:    // 批量端点
            break;
        case USB_EP_MODE_TYPE_INTR:    // 中断端点
            break;
        default:
            return;
    }

    // 配置IN端点
    if (epcp->in_state != NULL) {
        // 分配IN缓冲区
        epmo = usb_epmem_alloc(usbp, epcp->in_maxsize);
        cfgr |= epmo << 0;  // 缓冲区地址
        cfgr |= roundup2(epcp->in_maxsize, 4) << 10;  // 缓冲区长度
        cfgr |= USBEPnCFGR_EPDIR;  // IN方向
    }

    // 配置OUT端点
    if (epcp->out_state != NULL) {
        // 分配OUT缓冲区
        epmo = usb_epmem_alloc(usbp, epcp->out_maxsize);
        if (ep > 0) {  // EP0的OUT缓冲区已分配
            cfgr |= epmo << 0;  // 缓冲区地址
            cfgr |= roundup2(epcp->out_maxsize, 4) << 10;  // 缓冲区长度
        }
    }

    // 写入配置寄存器
    USB->EP[ep].CFGR = cfgr;
    
    // 使能端点中断
    USB->EP[ep].IER = (ep == 0) ?
        // EP0: SETUP接收 + IN传输 + OUT接收
        (USBEPnIER_SDRXIE | USBEPnIER_IDTXIE | USBEPnIER_ODRXIE) :
        // EP1-7: OUT接收 + IN传输
        (USBEPnIER_ODRXIE | USBEPnIER_IDTXIE);
    
    // 使能该端点的全局中断
    USB->IER |= (USBIER_EP0IE << ep);
}
```

### 端点配置寄存器(CFGR)字段说明

#### CFGR寄存器布局
```
位[31]      EPEN      端点使能
位[29]      EPTYPE    传输类型 (0=控制/批量/中断, 1=同步)
位[28]      EPDIR     方向 (0=OUT, 1=IN)
位[27:24]   EPADR     端点地址 (0-7)
位[16:10]   EPLEN     缓冲区长度 (4字节对齐)
位[9:0]     EPBUFA    缓冲区偏移地址 (相对USB_SRAM_BASE)
```

#### 配置示例
```c
// 配置EP1 IN批量端点，64字节
uint32_t cfgr = 
    USBEPnCFGR_EPEN |           // 使能端点
    (1 << 24) |                 // 端点地址 = 1
    (0x88 << 0) |               // 缓冲区地址 = 0x88
    (64 << 10) |                // 缓冲区长度 = 64
    USBEPnCFGR_EPDIR;           // IN方向
// 不设置EPTYPE，默认为批量/中断类型

USB->EP[1].CFGR = cfgr;
```

## 5. 应用层初始化流程

### 主程序初始化示例
```c
// main.c
int main(void) {
    // 1. HAL初始化
    halInit();
    
    // 2. 系统初始化
    chSysInit();
    
    // 3. 断开USB (确保干净状态)
    usbDisconnectBus(&USBD1);
    chThdSleepMilliseconds(1500);  // 等待主机识别断开
    
    // 4. 启动USB驱动
    usbStart(&USBD1, &usbcfg);
    
    // 5. 连接USB总线
    usbConnectBus(&USBD1);
    
    // 6. 进入主循环
    while(1) {
        chThdSleepSeconds(1);
    }
}
```

### USB配置结构
```c
const USBConfig usbcfg = {
    NULL,                    // USB事件回调 (可选)
    get_descriptor,          // GET_DESCRIPTOR回调 (必须)
    request_handler,         // 请求处理回调 (可选)
    NULL                     // SOF回调 (可选)
};
```

## 6. USB连接/断开操作

### 连接USB总线
```c
// 宏定义 (hal_usb_lld.h: 345-348行)
#define usb_lld_connect_bus(usbp) \
    do { \
        USB->CSR |= USBCSR_DPPUEN;  // 使能DP上拉
    } while (FALSE)

// 使用
usbConnectBus(&USBD1);
```

### 断开USB总线
```c
// 宏定义 (hal_usb_lld.h: 355-358行)
#define usb_lld_disconnect_bus(usbp) \
    do { \
        USB->CSR &= ~USBCSR_DPPUEN;  // 禁用DP上拉
    } while (FALSE)

// 使用
usbDisconnectBus(&USBD1);
```

### 主机唤醒
```c
// 宏定义 (hal_usb_lld.h: 365-368行)
#define usb_lld_wakeup_host(usbp) \
    do { \
        USB->CSR |= USBCSR_GENRSM;  // 生成恢复信号
    } while (FALSE)

// 使用
usb_lld_wakeup_host(&USBD1);
```

## 7. 完整初始化时序图

```
系统上电
    │
    ├─> halInit()
    │       └─> usb_lld_init()
    │               ├─ 配置USB时钟预分频器
    │               └─ 使能USB时钟
    │
    ├─> chSysInit()
    │
    ├─> usbDisconnectBus(&USBD1)
    │       └─> USB->CSR &= ~USBCSR_DPPUEN
    │
    ├─> 延时1500ms (等待主机识别断开)
    │
    ├─> usbStart(&USBD1, &usbcfg)
    │       └─> usb_lld_start()
    │               ├─ 使能USB中断
    │               ├─ 配置USB->CSR
    │               ├─ 清除中断标志
    │               └─ 使能USB->IER
    │
    └─> usbConnectBus(&USBD1)
            └─> USB->CSR |= USBCSR_DPPUEN
                    │
                    └─> 主机检测到设备
                            │
                            └─> 发送USB RESET
                                    │
                                    └─> usb_lld_reset()
                                            ├─ 清除CSR
                                            ├─ 初始化epmem_next
                                            ├─ 初始化EP0
                                            └─ 重新使能中断
```

## 8. halconf.h 和 mcuconf.h 配置

### halconf.h
```c
// 使能USB子系统
#define HAL_USE_USB         TRUE

// USB驱动选项
#define USB_USE_WAIT        TRUE  // 使能同步API
```

### mcuconf.h
```c
// USB驱动使能
#define HT32_USB_USE_USB0               TRUE

// USB中断优先级
#define HT32_USB_USB0_IRQ_PRIORITY      5

// USB时钟配置
#define HT32_USB_PRESCALER              3  // 144MHz / 3 = 48MHz
```

## 9. 常见问题和注意事项

### 问题1: USB时钟配置错误
**现象**: USB无法枚举，主机无法识别设备

**原因**: USB时钟不是48MHz

**解决**:
```c
// 确保PLL输出频率能被整除为48MHz
// 例如: PLL = 144MHz, 预分频器 = 3
// USB_CLK = 144MHz / 3 = 48MHz

#define HT32_USB_PRESCALER  3
```

### 问题2: 端点内存溢出
**现象**: 断言失败 "EPSRAM exhausted"

**原因**: 端点缓冲区总大小超过1024字节

**解决**:
```c
// 检查所有端点缓冲区大小
// EP0: 8 + 64 + 64 = 136字节
// EP1-7: 根据需要分配
// 总计不超过1024字节
```

### 问题3: 连接USB后立即崩溃
**现象**: usbConnectBus后系统崩溃

**原因**: 未在连接前调用usbStart()

**解决**:
```c
// 正确顺序
usbStart(&USBD1, &usbcfg);     // 先启动
usbConnectBus(&USBD1);         // 再连接
```

### 问题4: USB无法断开重连
**现象**: 第二次连接失败

**原因**: 断开后未等待足够时间

**解决**:
```c
usbDisconnectBus(&USBD1);
chThdSleepMilliseconds(1500);  // 至少等待1.5秒
usbStart(&USBD1, &usbcfg);
usbConnectBus(&USBD1);
```
