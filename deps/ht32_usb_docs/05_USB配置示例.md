# HT32F52352 USB配置示例

## 文件位置参考
- Demo配置: `ChibiOS-Contrib/demos/HT32/HT32F165x_USB_DFU/cfg/`
- 驱动代码: `ChibiOS-Contrib/os/hal/ports/HT32/LLD/USBv1/`

## 1. mcuconf.h 完整配置

### 基础系统配置
```c
#ifndef MCUCONF_H
#define MCUCONF_H

#define HT32F52352_MCUCONF

/*
 * HAL driver system settings.
 */

/*===========================================================================*/
/* Clock configuration.                                                      */
/*===========================================================================*/

/* 外部高速晶振频率 */
#define HT32_CK_HSE_FREQUENCY   8000000UL       // 8 MHz

/* 系统时钟源选择 */
#define HT32_CKCU_SW            CKCU_GCCR_SW_PLL  // 使用PLL

/* PLL配置 */
#define HT32_PLL_USE_HSE        TRUE              // PLL使用HSE
#define HT32_PLL_FBDIV          18                // PLL倍频系数
                                                   // PLL = HSE * FBDIV
                                                   // PLL = 8MHz * 18 = 144MHz
#define HT32_PLL_OTDIV          0                 // PLL输出分频

/* AHB时钟分频 */
#define HT32_AHB_PRESCALER      2                 // AHB = PLL / 2
                                                   // AHB = 144MHz / 2 = 72MHz

/* 外设时钟分频 */
#define HT32_USART_PRESCALER    1                 // USART时钟 = 72MHz

/* USB时钟分频 (关键配置！) */
#define HT32_USB_PRESCALER      3                 // USB = PLL / 3
                                                   // USB = 144MHz / 3 = 48MHz
                                                   // ★必须精确为48MHz★

/* SysTick配置 */
#define HT32_ST_USE_HCLK        TRUE              // SysTick使用HCLK (72MHz)

/*===========================================================================*/
/* USB driver settings                                                       */
/*===========================================================================*/

/* USB驱动使能 */
#define HT32_USB_USE_USB0                   TRUE

/* USB中断优先级 (0-15, 数字越小优先级越高) */
#define HT32_USB_USB0_IRQ_PRIORITY          5

/*===========================================================================*/
/* GPT driver settings                                                       */
/*===========================================================================*/

#define HT32_GPT_USE_BFTM0                  FALSE
#define HT32_GPT_BFTM0_IRQ_PRIORITY         4

/*===========================================================================*/
/* PWM driver settings                                                       */
/*===========================================================================*/

#define HT32_PWM_USE_GPTM1                  FALSE

#endif /* MCUCONF_H */
```

### 时钟配置计算示例
```c
/*
 * 时钟树:
 *
 * HSE (8MHz)
 *   │
 *   └─> PLL (×18) = 144MHz
 *         │
 *         ├─> USB (/3) = 48MHz ★
 *         │
 *         └─> AHB (/2) = 72MHz
 *               │
 *               ├─> CPU (72MHz)
 *               ├─> HCLK (72MHz)
 *               ├─> USART (/1) = 72MHz
 *               └─> APB外设 (72MHz)
 *
 * 关键要求:
 * - USB时钟必须为48MHz (±0.25%)
 * - AHB时钟最大72MHz
 * - PLL输出144-160MHz时稳定性最佳
 */
```

---

## 2. halconf.h 完整配置

```c
#ifndef HALCONF_H
#define HALCONF_H

#define _CHIBIOS_HAL_CONF_
#define _CHIBIOS_HAL_CONF_VER_8_4_

#include "mcuconf.h"

/*===========================================================================*/
/* Driver enable switches.                                                   */
/*===========================================================================*/

/* 基础驱动 */
#define HAL_USE_PAL                         TRUE  // GPIO

/* 通信驱动 */
#define HAL_USE_USB                         TRUE  // USB★
#define HAL_USE_SERIAL                      FALSE // 串口
#define HAL_USE_SPI                         FALSE // SPI
#define HAL_USE_I2C                         FALSE // I2C

/* 存储驱动 */
#define HAL_USE_SDC                         FALSE // SD卡
#define HAL_USE_MMC_SPI                     FALSE // MMC SPI

/* 定时器驱动 */
#define HAL_USE_GPT                         FALSE // 通用定时器
#define HAL_USE_PWM                         FALSE // PWM
#define HAL_USE_ICU                         FALSE // 输入捕获

/* 模拟驱动 */
#define HAL_USE_ADC                         FALSE // ADC
#define HAL_USE_DAC                         FALSE // DAC

/* 其他驱动 */
#define HAL_USE_RTC                         FALSE // RTC
#define HAL_USE_WDG                         FALSE // 看门狗
#define HAL_USE_CAN                         FALSE // CAN
#define HAL_USE_MAC                         FALSE // 以太网MAC
#define HAL_USE_UART                        FALSE // UART
#define HAL_USE_SERIAL_USB                  FALSE // USB串口
#define HAL_USE_CRY                         FALSE // 加密
#define HAL_USE_EFL                         FALSE // 嵌入式Flash
#define HAL_USE_I2S                         FALSE // I2S
#define HAL_USE_SIO                         FALSE // 串行IO
#define HAL_USE_TRNG                        FALSE // 真随机数
#define HAL_USE_WSPI                        FALSE // 宽带SPI

/*===========================================================================*/
/* USB driver related settings.                                              */
/*===========================================================================*/

/**
 * @brief   启用同步API
 * @details 允许使用阻塞式USB传输函数
 */
#if !defined(USB_USE_WAIT) || defined(__DOXYGEN__)
#define USB_USE_WAIT                        TRUE
#endif

/*===========================================================================*/
/* PAL driver related settings.                                              */
/*===========================================================================*/

#if !defined(PAL_USE_CALLBACKS) || defined(__DOXYGEN__)
#define PAL_USE_CALLBACKS                   FALSE
#endif

#if !defined(PAL_USE_WAIT) || defined(__DOXYGEN__)
#define PAL_USE_WAIT                        FALSE
#endif

/*===========================================================================*/
/* Other driver related settings.                                            */
/*===========================================================================*/

/* 其他驱动配置保持默认值 */

#endif /* HALCONF_H */
```

---

## 3. USB设备描述符配置

### USB设备描述符
```c
// 设备描述符 (18字节)
static const uint8_t device_descriptor_data[18] = {
    USB_DESC_DEVICE(
        0x0200,        // bcdUSB (USB 2.0)
        0x00,          // bDeviceClass (在接口中定义)
        0x00,          // bDeviceSubClass
        0x00,          // bDeviceProtocol
        64,            // bMaxPacketSize0 (EP0最大包大小)
        0x04d9,        // idVendor (厂商ID) ★修改为你的VID★
        0xf00d,        // idProduct (产品ID) ★修改为你的PID★
        0x0200,        // bcdDevice (设备版本)
        1,             // iManufacturer (厂商字符串索引)
        2,             // iProduct (产品字符串索引)
        3,             // iSerialNumber (序列号字符串索引)
        1              // bNumConfigurations (配置数量)
    )
};

static const USBDescriptor device_descriptor = {
    sizeof(device_descriptor_data),
    device_descriptor_data
};
```

### USB配置描述符
```c
// 配置描述符 + 接口描述符 + 端点描述符
static const uint8_t configuration_descriptor_data[] = {
    // 配置描述符 (9字节)
    USB_DESC_CONFIGURATION(
        9 + 9 + 7 + 7,  // wTotalLength (配置+接口+2个端点)
        0x01,           // bNumInterfaces (接口数量)
        0x01,           // bConfigurationValue (配置值)
        0,              // iConfiguration (配置字符串索引)
        0x80,           // bmAttributes (总线供电)
        50              // bMaxPower (100mA)
    ),
    
    // 接口描述符 (9字节)
    USB_DESC_INTERFACE(
        0x00,           // bInterfaceNumber
        0x00,           // bAlternateSetting
        0x02,           // bNumEndpoints (端点数量，不包括EP0)
        0xFF,           // bInterfaceClass (厂商自定义)
        0x00,           // bInterfaceSubClass
        0x00,           // bInterfaceProtocol
        0               // iInterface (接口字符串索引)
    ),
    
    // 端点1 IN描述符 (7字节)
    USB_DESC_ENDPOINT(
        0x81,           // bEndpointAddress (EP1 IN)
        USB_EP_MODE_TYPE_BULK,  // bmAttributes (批量传输)
        64,             // wMaxPacketSize (64字节)
        0               // bInterval (批量传输忽略)
    ),
    
    // 端点1 OUT描述符 (7字节)
    USB_DESC_ENDPOINT(
        0x01,           // bEndpointAddress (EP1 OUT)
        USB_EP_MODE_TYPE_BULK,  // bmAttributes (批量传输)
        64,             // wMaxPacketSize (64字节)
        0               // bInterval (批量传输忽略)
    )
};

static const USBDescriptor configuration_descriptor = {
    sizeof(configuration_descriptor_data),
    configuration_descriptor_data
};
```

### USB字符串描述符
```c
// 语言ID字符串 (字符串0)
static const uint8_t string0[] = {
    USB_DESC_BYTE(4),                     // bLength
    USB_DESC_BYTE(USB_DESCRIPTOR_STRING), // bDescriptorType
    USB_DESC_WORD(0x0409)                 // wLANGID (美国英语)
};

// 厂商字符串 (字符串1)
static const uint8_t string1[] = {
    USB_DESC_BYTE(28),                    // bLength
    USB_DESC_BYTE(USB_DESCRIPTOR_STRING),
    'M', 0, 'y', 0, ' ', 0, 'C', 0, 'o', 0, 'm', 0, 'p', 0, 'a', 0,
    'n', 0, 'y', 0, ' ', 0, 'L', 0, 't', 0, 'd', 0
};

// 产品字符串 (字符串2)
static const uint8_t string2[] = {
    USB_DESC_BYTE(30),
    USB_DESC_BYTE(USB_DESCRIPTOR_STRING),
    'M', 0, 'y', 0, ' ', 0, 'U', 0, 'S', 0, 'B', 0, ' ', 0, 
    'D', 0, 'e', 0, 'v', 0, 'i', 0, 'c', 0, 'e', 0, ' ', 0
};

// 序列号字符串 (字符串3)
static const uint8_t string3[] = {
    USB_DESC_BYTE(18),
    USB_DESC_BYTE(USB_DESCRIPTOR_STRING),
    '0', 0, '0', 0, '0', 0, '0', 0, '0', 0, '0', 0, '0', 0, '1', 0
};

// 字符串数组
static const USBDescriptor strings[] = {
    {sizeof(string0), string0},
    {sizeof(string1), string1},
    {sizeof(string2), string2},
    {sizeof(string3), string3}
};
```

---

## 4. USB回调函数配置

### GET_DESCRIPTOR回调
```c
static const USBDescriptor *get_descriptor(USBDriver *usbp,
                                           uint8_t dtype,
                                           uint8_t dindex,
                                           uint16_t lang) {
    (void)usbp;
    (void)lang;
    
    switch (dtype) {
    case USB_DESCRIPTOR_DEVICE:
        return &device_descriptor;
        
    case USB_DESCRIPTOR_CONFIGURATION:
        return &configuration_descriptor;
        
    case USB_DESCRIPTOR_STRING:
        if (dindex < 4)
            return &strings[dindex];
        break;
    }
    
    return NULL;
}
```

### 请求处理回调
```c
static bool request_handler(USBDriver *usbp) {
    const USBSetup *setup = (const USBSetup *)usbp->setup;
    
    // 处理标准请求
    if ((setup->bmRequestType & USB_RTYPE_TYPE_MASK) == USB_RTYPE_TYPE_STANDARD) {
        // 标准请求由USB栈自动处理
        return false;
    }
    
    // 处理类特定请求
    if ((setup->bmRequestType & USB_RTYPE_TYPE_MASK) == USB_RTYPE_TYPE_CLASS) {
        // 处理类特定请求
        switch (setup->bRequest) {
        case MY_CLASS_REQUEST:
            // 处理请求
            usbSetupTransfer(usbp, response_data, response_size, NULL);
            return true;
        }
    }
    
    // 处理厂商特定请求
    if ((setup->bmRequestType & USB_RTYPE_TYPE_MASK) == USB_RTYPE_TYPE_VENDOR) {
        // 处理厂商特定请求
        switch (setup->bRequest) {
        case MY_VENDOR_REQUEST:
            // 处理请求
            usbSetupTransfer(usbp, vendor_data, vendor_size, NULL);
            return true;
        }
    }
    
    return false;  // 未处理的请求
}
```

### USB事件回调
```c
static void usb_event(USBDriver *usbp, usbevent_t event) {
    switch (event) {
    case USB_EVENT_ADDRESS:
        // 地址已设置
        break;
        
    case USB_EVENT_CONFIGURED:
        // 设备已配置
        chSysLockFromISR();
        // 启动端点
        usbInitEndpointI(usbp, DATA_REQUEST_EP, &ep_config);
        // 开始接收
        usbStartReceiveI(usbp, DATA_REQUEST_EP, rx_buffer, sizeof(rx_buffer));
        chSysUnlockFromISR();
        break;
        
    case USB_EVENT_RESET:
        // USB复位
        break;
        
    case USB_EVENT_SUSPEND:
        // USB挂起
        break;
        
    case USB_EVENT_WAKEUP:
        // 从挂起恢复
        break;
        
    case USB_EVENT_STALLED:
        // 端点STALL
        break;
    }
}
```

### USB配置结构
```c
const USBConfig usbcfg = {
    usb_event,          // USB事件回调
    get_descriptor,     // GET_DESCRIPTOR回调
    request_handler,    // 请求处理回调
    NULL                // SOF回调 (可选)
};
```

---

## 5. USB端点配置

### EP0配置 (由驱动自动配置)
```c
// EP0配置在usb_lld_reset()中自动完成
// 应用层不需要配置EP0
```

### 数据端点配置示例
```c
// EP1 IN端点状态
static USBInEndpointState ep1_in_state;

// EP1 OUT端点状态
static USBOutEndpointState ep1_out_state;

// EP1 IN回调
static void ep1_in_cb(USBDriver *usbp, usbep_t ep) {
    (void)usbp;
    (void)ep;
    
    // IN传输完成
    // 处理发送完成事件
}

// EP1 OUT回调
static void ep1_out_cb(USBDriver *usbp, usbep_t ep) {
    size_t n = usb_lld_get_transaction_size(usbp, ep);
    
    // OUT传输完成
    // 处理接收到的数据
    process_received_data(rx_buffer, n);
    
    // 继续接收
    usbStartReceiveI(usbp, ep, rx_buffer, sizeof(rx_buffer));
}

// EP1配置
static const USBEndpointConfig ep1_config = {
    USB_EP_MODE_TYPE_BULK,   // 批量端点
    NULL,                    // 无SETUP回调
    ep1_in_cb,               // IN回调
    ep1_out_cb,              // OUT回调
    64,                      // IN最大包: 64字节
    64,                      // OUT最大包: 64字节
    &ep1_in_state,           // IN状态
    &ep1_out_state,          // OUT状态
    1,                       // 缓冲区数量
    NULL                     // 无SETUP缓冲区
};

// 初始化EP1 (在USB_EVENT_CONFIGURED中调用)
usbInitEndpointI(&USBD1, 1, &ep1_config);
```

---

## 6. 主程序配置

### 完整main.c示例
```c
#include "ch.h"
#include "hal.h"
#include "usb_config.h"  // 包含USB描述符和配置

// USB缓冲区
static uint8_t rx_buffer[64];
static uint8_t tx_buffer[64];

/*
 * 应用入口
 */
int main(void) {
    /*
     * 系统初始化
     * - HAL初始化，内部启用caches
     * - 内核初始化，main()变成线程，RTOS激活
     */
    halInit();
    chSysInit();

    /*
     * USB初始化序列
     */
    // 1. 断开USB (确保干净状态)
    usbDisconnectBus(&USBD1);
    chThdSleepMilliseconds(1500);

    // 2. 启动USB驱动
    usbStart(&USBD1, &usbcfg);

    // 3. 连接USB总线
    usbConnectBus(&USBD1);

    /*
     * 正常线程活动
     */
    while (true) {
        // 主循环
        chThdSleepMilliseconds(1000);
    }
}
```

---

## 7. 数据传输示例

### 批量数据发送
```c
// 发送数据函数
void send_data(const uint8_t *data, size_t size) {
    // 阻塞式发送
    usbTransmit(&USBD1, DATA_EP, data, size);
    
    // 或者非阻塞式发送
    // usbStartTransmitI(&USBD1, DATA_EP, data, size);
    // 在ep_in_cb中处理完成事件
}

// 使用示例
uint8_t message[] = "Hello USB!";
send_data(message, sizeof(message) - 1);
```

### 批量数据接收
```c
// 启动接收
void start_receive(void) {
    usbStartReceiveI(&USBD1, DATA_EP, rx_buffer, sizeof(rx_buffer));
}

// 在ep_out_cb中处理接收到的数据
static void ep_out_cb(USBDriver *usbp, usbep_t ep) {
    size_t n = usb_lld_get_transaction_size(usbp, ep);
    
    // 处理数据
    for (size_t i = 0; i < n; i++) {
        // 处理每个字节
        process_byte(rx_buffer[i]);
    }
    
    // 继续接收
    usbStartReceiveI(usbp, ep, rx_buffer, sizeof(rx_buffer));
}
```

---

## 8. 调试配置

### 添加调试输出
```c
// 使用串口输出调试信息 (如果有可用串口)
#define DEBUG_USB  1

#if DEBUG_USB
    #define USB_DEBUG(fmt, ...) chprintf((BaseSequentialStream *)&SD1, fmt, ##__VA_ARGS__)
#else
    #define USB_DEBUG(fmt, ...)
#endif

// 使用示例
USB_DEBUG("USB: Device configured\r\n");
USB_DEBUG("USB: EP%d IN complete, sent %d bytes\r\n", ep, n);
```

### 添加状态LED
```c
// LED配置
#define LED_USB_ACTIVE    PAL_LINE(GPIOB, 0)

// 在USB事件中切换LED
static void usb_event(USBDriver *usbp, usbevent_t event) {
    switch (event) {
    case USB_EVENT_CONFIGURED:
        palSetLine(LED_USB_ACTIVE);  // LED亮
        break;
        
    case USB_EVENT_SUSPEND:
        palClearLine(LED_USB_ACTIVE);  // LED灭
        break;
    }
}
```

---

## 9. 编译配置

### Makefile配置
```makefile
# USB相关源文件
USBSRC = $(CHIBIOS_CONTRIB)/os/hal/ports/HT32/LLD/USBv1/hal_usb_lld.c

# USB描述符源文件
USBSRC += usb_config.c

# 添加到编译列表
CSRC += $(USBSRC)

# USB相关头文件路径
USBINC = $(CHIBIOS_CONTRIB)/os/hal/ports/HT32/LLD/USBv1

# 添加到包含路径
INCDIR += $(USBINC)
```

---

## 10. 常见配置错误

### 错误1: USB时钟不正确
```c
// 错误配置
#define HT32_USB_PRESCALER  2  // 144MHz / 2 = 72MHz ❌

// 正确配置
#define HT32_USB_PRESCALER  3  // 144MHz / 3 = 48MHz ✓
```

### 错误2: 端点缓冲区溢出
```c
// 错误: 总缓冲区超过1024字节
// EP0: 136字节
// EP1: 512字节
// EP2: 512字节
// 总计: 1160字节 ❌

// 正确: 合理分配缓冲区
// EP0: 136字节
// EP1: 256字节
// EP2: 256字节
// EP3: 256字节
// 总计: 904字节 ✓
```

### 错误3: VID/PID冲突
```c
// 错误: 使用其他厂商的VID/PID
#define MY_VID  0x04d9  // 这是别人的VID! ❌

// 正确: 申请自己的VID或使用测试VID
#define MY_VID  0x16C0  // USB-IF分配的测试VID ✓
#define MY_PID  0x05DC  // 测试PID
```

### 错误4: 中断优先级配置错误
```c
// 错误: USB中断优先级低于其他关键中断
#define HT32_USB_USB0_IRQ_PRIORITY  15  // 优先级太低 ❌

// 正确: 给USB合适的优先级
#define HT32_USB_USB0_IRQ_PRIORITY  5   // 中等优先级 ✓
```

---

## 完整项目文件结构

```
my_usb_project/
├── main.c                  # 主程序
├── usb_config.c            # USB配置和描述符
├── usb_config.h            # USB配置头文件
├── halconf.h               # HAL配置
├── mcuconf.h               # MCU配置
├── chconf.h                # ChibiOS配置
├── Makefile                # 编译配置
└── board/
    ├── board.c             # 板级支持包
    └── board.h
```

---

**配置文件说明完成**

> 📌 **提示**: 以上配置可以直接用于HT32F52352项目开发。建议先从简单的USB设备（如CDC虚拟串口）开始，逐步增加复杂功能。
