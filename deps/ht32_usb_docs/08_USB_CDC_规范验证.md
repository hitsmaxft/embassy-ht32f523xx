# USB CDC 规范验证指南

## 概述
本文档基于 **USB CDC (Communication Device Class) v1.2 规范** 和 **USB ACM (Abstract Control Model) 子类规范**，提供完整的合规性检查清单，确保 HT32F52352 上的 Embassy 实现符合标准。

---

## 1. USB CDC 规范要求对照

### 1.1 设备类定义 (Device Class)

**规范要求**:
- bDeviceClass = 0x02 (Communication Device Class)
- bDeviceSubClass = 0x00 (未使用，在接口级别定义)
- bDeviceProtocol = 0x00 (未使用，在接口级别定义)

**实现验证**:
```c
// 设备描述符检查
static const uint8_t device_descriptor[18] = {
    0x12,        // bLength = 18
    0x01,        // bDescriptorType = Device
    0x00, 0x02,  // bcdUSB = USB 2.0
    0x02,        // bDeviceClass = CDC ✓
    0x00,        // bDeviceSubClass = 0 ✓
    0x00,        // bDeviceProtocol = 0 ✓
    0x40,        // bMaxPacketSize0 = 64
    // ... VID/PID/版本/字符串索引
    0x01         // bNumConfigurations = 1
};
```

**Embassy 实现检查**:
```rust
// 确保 Embassy 配置正确的设备类
let mut config = Config::new(vid, pid);
config.device_class = 0x02;        // CDC
config.device_sub_class = 0x00;
config.device_protocol = 0x00;
config.max_packet_size_0 = 64;     // 必须是 8, 16, 32, 或 64
```

### 1.2 接口关联描述符 (Interface Association Descriptor)

**规范要求 (USB IAD v1.0)**:
CDC 复合设备必须使用 IAD 来关联通信接口和数据接口。

```c
// 接口关联描述符 (8字节)
static const uint8_t interface_association_descriptor[] = {
    0x08,        // bLength = 8
    0x0B,        // bDescriptorType = Interface Association
    0x00,        // bFirstInterface = 0 (通信接口)
    0x02,        // bInterfaceCount = 2 (通信+数据接口)
    0x02,        // bFunctionClass = CDC
    0x02,        // bFunctionSubClass = ACM
    0x00,        // bFunctionProtocol = 0
    0x00         // iFunction = 0 (无字符串)
};
```

**关键验证点**:
- ✓ bDescriptorType = 0x0B (Interface Association)
- ✓ bInterfaceCount = 2 (必须关联两个接口)
- ✓ bFunctionClass = 0x02 (CDC)
- ✓ bFunctionSubClass = 0x02 (ACM)

### 1.3 通信接口 (Communication Interface)

**规范要求**:
- bInterfaceClass = 0x02 (Communication Interface Class)
- bInterfaceSubClass = 0x02 (Abstract Control Model)
- bInterfaceProtocol = 0x00 或 0x01 (AT Commands 或 Vendor Specific)
- 必须有一个中断 IN 端点

```c
// 通信接口描述符 (9字节)
static const uint8_t comm_interface_descriptor[] = {
    0x09,        // bLength = 9
    0x04,        // bDescriptorType = Interface
    0x00,        // bInterfaceNumber = 0
    0x00,        // bAlternateSetting = 0
    0x01,        // bNumEndpoints = 1 (中断端点)
    0x02,        // bInterfaceClass = Communication Interface Class ✓
    0x02,        // bInterfaceSubClass = Abstract Control Model ✓
    0x00,        // bInterfaceProtocol = 0 (无特定协议) ✓
    0x00         // iInterface = 0
};
```

### 1.4 CDC 功能描述符 (Functional Descriptors)

**规范要求**: CDC ACM 必须包含以下功能描述符

#### 1.4.1 头功能描述符 (Header Functional Descriptor)
```c
static const uint8_t cdc_header_descriptor[] = {
    0x05,        // bFunctionLength = 5
    0x24,        // bDescriptorType = CS_INTERFACE ✓
    0x00,        // bDescriptorSubtype = Header ✓
    0x10, 0x01   // bcdCDC = CDC 1.1 ✓
};
```

#### 1.4.2 ACM 功能描述符 (Abstract Control Management)
```c
static const uint8_t cdc_acm_descriptor[] = {
    0x04,        // bFunctionLength = 4
    0x24,        // bDescriptorType = CS_INTERFACE ✓
    0x02,        // bDescriptorSubtype = Abstract Control Management ✓
    0x06         // bmCapabilities = 位图能力 ✓
                 // bit 1: 支持 Set_Line_Coding, Set_Control_Line_State,
                 //        Get_Line_Coding, Serial_State 通知
                 // bit 2: 支持 Send_Break
};
```

**bmCapabilities 位定义**:
- bit 0: 支持网络连接通知 (不适用于串口)
- bit 1: 支持 line coding 和 control line state 管理 ✓
- bit 2: 支持 Send_Break ✓
- bit 3: 支持网络连接管理 (不适用)

#### 1.4.3 联合功能描述符 (Union Functional Descriptor)
```c
static const uint8_t cdc_union_descriptor[] = {
    0x05,        // bFunctionLength = 5
    0x24,        // bDescriptorType = CS_INTERFACE ✓
    0x06,        // bDescriptorSubtype = Union ✓
    0x00,        // bControlInterface = 0 (通信接口) ✓
    0x01         // bSubordinateInterface0 = 1 (数据接口) ✓
};
```

### 1.5 数据接口 (Data Interface)

**规范要求**:
- bInterfaceClass = 0x0A (Data Interface Class)
- bInterfaceSubClass = 0x00 (未使用)
- bInterfaceProtocol = 0x00 (无特定协议)
- 必须有一个批量 IN 端点和一个批量 OUT 端点

```c
// 数据接口描述符 (9字节)
static const uint8_t data_interface_descriptor[] = {
    0x09,        // bLength = 9
    0x04,        // bDescriptorType = Interface
    0x01,        // bInterfaceNumber = 1
    0x00,        // bAlternateSetting = 0
    0x02,        // bNumEndpoints = 2 (批量 IN + OUT)
    0x0A,        // bInterfaceClass = Data Interface Class ✓
    0x00,        // bInterfaceSubClass = 0 ✓
    0x00,        // bInterfaceProtocol = 0 ✓
    0x00         // iInterface = 0
};
```

### 1.6 端点要求

#### 1.6.1 通信端点 (中断 IN)
```c
static const uint8_t comm_endpoint_descriptor[] = {
    0x07,        // bLength = 7
    0x05,        // bDescriptorType = Endpoint
    0x81,        // bEndpointAddress = EP1 IN ✓
    0x03,        // bmAttributes = Interrupt ✓
    0x40, 0x00,  // wMaxPacketSize = 64 (≤ 64 for Full-Speed) ✓
    0x10         // bInterval = 16ms (1-255ms for Full-Speed) ✓
};
```

**规范验证**:
- ✓ 方向必须是 IN (0x80 | endpoint_number)
- ✓ 传输类型必须是中断 (0x03)
- ✓ 最大包大小 ≤ 64 字节 (Full-Speed)
- ✓ 轮询间隔 1-255ms (Full-Speed)

#### 1.6.2 数据端点 (批量 OUT/IN)
```c
// 批量 OUT 端点
static const uint8_t data_out_endpoint_descriptor[] = {
    0x07,        // bLength = 7
    0x05,        // bDescriptorType = Endpoint
    0x02,        // bEndpointAddress = EP2 OUT ✓
    0x02,        // bmAttributes = Bulk ✓
    0x40, 0x00,  // wMaxPacketSize = 64 ✓
    0x00         // bInterval = 0 (忽略批量端点) ✓
};

// 批量 IN 端点
static const uint8_t data_in_endpoint_descriptor[] = {
    0x07,        // bLength = 7
    0x05,        // bDescriptorType = Endpoint
    0x82,        // bEndpointAddress = EP2 IN ✓
    0x02,        // bmAttributes = Bulk ✓
    0x40, 0x00,  // wMaxPacketSize = 64 ✓
    0x00         // bInterval = 0 ✓
};
```

**规范验证**:
- ✓ OUT端点: 方向必须是 OUT (endpoint_number)
- ✓ IN端点: 方向必须是 IN (0x80 | endpoint_number)
- ✓ 传输类型必须是批量 (0x02)
- ✓ Full-Speed 最大包大小: 8, 16, 32, 或 64 字节
- ✓ bInterval = 0 (批量端点忽略此值)

---

## 2. CDC ACM 类请求验证

### 2.1 必需的类请求

**规范要求**: CDC ACM 设备必须支持以下请求

#### 2.1.1 SET_LINE_CODING (0x20)
```c
typedef struct {
    uint32_t dwDTERate;      // 比特率 (如 115200)
    uint8_t bCharFormat;     // 停止位: 0=1位, 1=1.5位, 2=2位
    uint8_t bParityType;     // 奇偶校验: 0=无, 1=奇, 2=偶, 3=标记, 4=空格
    uint8_t bDataBits;       // 数据位: 5, 6, 7, 8, 或 16
} __packed line_coding_t;

// 请求格式验证
// bmRequestType = 0x21 (Host-to-Device, Class, Interface)
// bRequest = 0x20
// wValue = 0
// wIndex = 通信接口号 (0)
// wLength = 7
// Data = line_coding_t 结构

void handle_set_line_coding(uint8_t *setup) {
    if (setup[0] == 0x21 && setup[1] == 0x20 &&
        setup[4] == 0x00 && setup[6] == 0x07) {
        // 准备接收 7 字节数据
        prepare_control_out(7);
    }
}
```

#### 2.1.2 GET_LINE_CODING (0x21)
```c
// 请求格式验证
// bmRequestType = 0xA1 (Device-to-Host, Class, Interface)
// bRequest = 0x21
// wValue = 0
// wIndex = 通信接口号 (0)
// wLength = 7
// Data = 返回当前 line_coding_t

void handle_get_line_coding(uint8_t *setup) {
    if (setup[0] == 0xA1 && setup[1] == 0x21 &&
        setup[4] == 0x00 && setup[6] == 0x07) {
        // 发送当前线路编码
        send_control_data((uint8_t*)&current_line_coding, 7);
    }
}
```

#### 2.1.3 SET_CONTROL_LINE_STATE (0x22)
```c
// 请求格式验证
// bmRequestType = 0x21 (Host-to-Device, Class, Interface)
// bRequest = 0x22
// wValue = 控制信号状态位图
//   bit 0: DTR (Data Terminal Ready)
//   bit 1: RTS (Request To Send)
// wIndex = 通信接口号 (0)
// wLength = 0

void handle_set_control_line_state(uint8_t *setup) {
    if (setup[0] == 0x21 && setup[1] == 0x22 && setup[6] == 0x00) {
        uint16_t control_state = *(uint16_t*)&setup[2];
        bool dtr = (control_state & 0x01) != 0;
        bool rts = (control_state & 0x02) != 0;

        // 更新控制线状态
        update_control_lines(dtr, rts);

        // 发送 ACK
        send_control_status();
    }
}
```

### 2.2 可选的类请求

#### 2.2.1 SEND_BREAK (0x23)
如果 ACM 功能描述符中设置了 bit 2，则必须支持此请求。

```c
// 请求格式
// bmRequestType = 0x21
// bRequest = 0x23
// wValue = 中断持续时间 (ms)
// wIndex = 通信接口号
// wLength = 0

void handle_send_break(uint8_t *setup) {
    if (setup[0] == 0x21 && setup[1] == 0x23 && setup[6] == 0x00) {
        uint16_t duration = *(uint16_t*)&setup[2];

        // 实现发送中断信号
        send_break_signal(duration);

        send_control_status();
    }
}
```

### 2.3 通知 (Notifications)

**规范要求**: CDC ACM 设备可以发送异步通知到中断端点

#### 2.3.1 SERIAL_STATE 通知 (0x20)
```c
typedef struct {
    uint8_t bmRequestType;   // 0xA1 (Device-to-Host, Class, Interface)
    uint8_t bNotification;   // 0x20 (SERIAL_STATE)
    uint16_t wValue;         // 0
    uint16_t wIndex;         // 接口号
    uint16_t wLength;        // 2 (数据长度)
    uint16_t data;           // 串行状态位图
} __packed serial_state_notification_t;

// 串行状态位定义
#define SERIAL_STATE_DCD    (1 << 0)  // Data Carrier Detect
#define SERIAL_STATE_DSR    (1 << 1)  // Data Set Ready
#define SERIAL_STATE_BREAK  (1 << 2)  // Break detection
#define SERIAL_STATE_RI     (1 << 3)  // Ring Indicator
#define SERIAL_STATE_FRAME  (1 << 4)  // Framing error
#define SERIAL_STATE_PARITY (1 << 5)  // Parity error
#define SERIAL_STATE_OVERUN (1 << 6)  // Data overrun

void send_serial_state_notification(uint16_t state) {
    serial_state_notification_t notif = {
        .bmRequestType = 0xA1,
        .bNotification = 0x20,
        .wValue = 0,
        .wIndex = 0,  // 通信接口号
        .wLength = 2,
        .data = state
    };

    // 发送到中断端点
    send_interrupt_data((uint8_t*)&notif, sizeof(notif));
}
```

---

## 3. 数据传输规范验证

### 3.1 批量传输要求

**规范要求**:
- 数据传输使用批量端点
- 支持任意长度的数据包
- 短包 (< wMaxPacketSize) 表示传输结束
- 零长度包 (ZLP) 用于结束精确倍数包大小的传输

#### 3.1.1 OUT 传输 (接收)
```c
void validate_bulk_out_transfer() {
    // 1. 检查端点配置
    assert((USB->EP[2].CFGR & USBEPnCFGR_EPEN) != 0);     // 端点使能
    assert((USB->EP[2].CFGR & USBEPnCFGR_EPDIR) == 0);    // OUT 方向

    // 2. 接收数据处理
    if (USB->EP[2].ISR & USBEPnISR_ODRXIF) {
        size_t received = USB->EP[2].TCR & USBEPnTCR_TCNT;

        // 3. 规范验证
        assert(received <= 64);  // 不能超过 wMaxPacketSize

        // 4. 短包检测
        bool is_short_packet = (received < 64);
        bool transfer_complete = is_short_packet || (expected_length == 0);

        // 5. 处理数据...

        // 6. 准备接收下一个包 (如果传输未完成)
        if (!transfer_complete) {
            USB->EP[2].CSR &= ~USBEPnCSR_NAKRX;
        }
    }
}
```

#### 3.1.2 IN 传输 (发送)
```c
void validate_bulk_in_transfer(const uint8_t *data, size_t length) {
    size_t remaining = length;
    size_t offset = 0;

    while (remaining > 0) {
        // 1. 计算包大小
        size_t packet_size = (remaining > 64) ? 64 : remaining;

        // 2. 等待端点就绪
        while (USB->EP[2].CSR & USBEPnCSR_NAKTX) {
            // 等待上一个包发送完成
        }

        // 3. 传输数据...

        remaining -= packet_size;
        offset += packet_size;
    }

    // 4. 零长度包处理 (规范要求)
    if (length % 64 == 0) {
        // 发送 ZLP 表示传输结束
        send_zero_length_packet();
    }
}
```

### 3.2 流量控制

**规范建议**: 使用 DTR/RTS 信号进行流量控制

```c
typedef struct {
    bool dtr_asserted;    // Data Terminal Ready
    bool rts_asserted;    // Request To Send
    bool dcd_state;       // Data Carrier Detect
    bool dsr_state;       // Data Set Ready
} flow_control_state_t;

void handle_flow_control() {
    // 1. DTR 去断言时暂停发送
    if (!flow_control.dtr_asserted) {
        suspend_data_transmission();
    }

    // 2. RTS 去断言时暂停接收
    if (!flow_control.rts_asserted) {
        // 设置接收 NAK
        USB->EP[2].CSR |= USBEPnCSR_NAKRX;
    } else {
        // 恢复接收
        USB->EP[2].CSR &= ~USBEPnCSR_NAKRX;
    }

    // 3. 更新状态并发送通知
    uint16_t serial_state = 0;
    if (flow_control.dcd_state) serial_state |= SERIAL_STATE_DCD;
    if (flow_control.dsr_state) serial_state |= SERIAL_STATE_DSR;

    send_serial_state_notification(serial_state);
}
```

---

## 4. Embassy 实现规范合规性检查

### 4.1 描述符验证

```rust
// Embassy 配置验证
fn validate_embassy_cdc_config() {
    // 1. 设备类验证
    assert_eq!(config.device_class, 0x02);      // CDC
    assert_eq!(config.device_sub_class, 0x00);
    assert_eq!(config.device_protocol, 0x00);

    // 2. CDC 类配置
    let mut class = CdcAcmClass::new(&mut builder, &mut state, 64);

    // 3. 验证生成的描述符
    let descriptors = builder.build_descriptors();

    // 检查配置描述符长度 (应该 ≥ 67 字节)
    assert!(descriptors.configuration.len() >= 67);

    // 检查是否包含 IAD
    assert!(descriptors.configuration.contains(&[0x08, 0x0B])); // IAD

    // 检查功能描述符
    assert!(descriptors.configuration.contains(&[0x05, 0x24, 0x00])); // Header
    assert!(descriptors.configuration.contains(&[0x04, 0x24, 0x02])); // ACM
    assert!(descriptors.configuration.contains(&[0x05, 0x24, 0x06])); // Union
}
```

### 4.2 类请求处理验证

```rust
// Embassy 类请求处理
async fn handle_class_requests(class: &mut CdcAcmClass<'_>) {
    loop {
        match class.wait_class_request().await {
            ClassRequest::SetLineCoding(line_coding) => {
                // 验证参数合法性
                validate_line_coding(&line_coding);
                class.accept_class_request().await;
            }
            ClassRequest::GetLineCoding => {
                let current = get_current_line_coding();
                class.send_line_coding(current).await;
            }
            ClassRequest::SetControlLineState(dtr, rts) => {
                update_control_lines(dtr, rts);
                class.accept_class_request().await;
            }
            _ => {
                // 不支持的请求
                class.reject_class_request().await;
            }
        }
    }
}

fn validate_line_coding(coding: &LineCoding) {
    // 验证波特率范围
    assert!(coding.data_rate >= 300 && coding.data_rate <= 3000000);

    // 验证数据位
    assert!(matches!(coding.data_bits, 5 | 6 | 7 | 8 | 16));

    // 验证奇偶校验
    assert!(coding.parity_type <= 4);

    // 验证停止位
    assert!(coding.stop_bits <= 2);
}
```

### 4.3 数据传输验证

```rust
// 数据传输合规性验证
async fn validate_data_transfer(class: &mut CdcAcmClass<'_>) {
    let mut buffer = [0u8; 64];

    // 接收验证
    loop {
        let n = class.read(&mut buffer).await;

        // 验证接收数据长度
        assert!(n <= 64);  // 不能超过端点大小

        if n > 0 {
            process_received_data(&buffer[..n]);
        }
    }
}

async fn send_data_with_validation(class: &mut CdcAcmClass<'_>, data: &[u8]) {
    let mut offset = 0;

    while offset < data.len() {
        let chunk_size = core::cmp::min(64, data.len() - offset);
        let chunk = &data[offset..offset + chunk_size];

        // 发送数据块
        class.write(chunk).await;

        offset += chunk_size;
    }

    // 如果总长度是端点大小的倍数，Embassy 应该自动发送 ZLP
    // 验证这一行为
    if data.len() % 64 == 0 {
        // Embassy 应该已经处理了 ZLP
        // 可以通过 USB 分析仪验证
    }
}
```

---

## 5. 测试和验证工具

### 5.1 USB 分析仪验证

使用 USB 分析仪 (如 Wireshark + USBPcap) 验证：

1. **枚举序列验证**:
   - GET_DESCRIPTOR (Device) → 18字节设备描述符
   - GET_DESCRIPTOR (Configuration) → 67字节配置描述符
   - SET_CONFIGURATION (1) → ACK

2. **描述符内容验证**:
   - 检查 IAD 是否存在且正确
   - 验证 CDC 功能描述符
   - 确认端点配置

3. **类请求验证**:
   - SET_LINE_CODING → 7字节数据
   - GET_LINE_CODING → 返回7字节
   - SET_CONTROL_LINE_STATE → 状态位

4. **数据传输验证**:
   - 批量传输的包结构
   - 零长度包的正确使用
   - 流量控制信号

### 5.2 主机端测试

```python
# Python 测试脚本 (使用 pyserial)
import serial
import time

def test_cdc_compliance():
    # 1. 打开 CDC 设备
    ser = serial.Serial('/dev/ttyACM0', 115200, timeout=1)

    # 2. 测试基本通信
    test_data = b'Hello, HT32!'
    ser.write(test_data)

    # 3. 读取回显
    response = ser.read(len(test_data))
    assert response == test_data

    # 4. 测试流量控制
    ser.dtr = False  # 断言 DTR
    time.sleep(0.1)
    ser.dtr = True   # 释放 DTR

    # 5. 测试不同波特率
    for baud in [9600, 19200, 38400, 57600, 115200]:
        ser.baudrate = baud
        ser.write(b'Test\n')
        response = ser.readline()
        assert b'Test' in response

    ser.close()
    print("CDC 合规性测试通过")

if __name__ == "__main__":
    test_cdc_compliance()
```

### 5.3 Windows 驱动验证

确保设备能被 Windows 识别为标准 CDC ACM 设备：

```inf
; Windows 应该自动使用内置的 usbser.sys 驱动
; 检查设备管理器中的设备状态
; VID/PID 应该出现在 "端口 (COM 和 LPT)" 下
```

---

## 6. 常见合规性问题和解决方案

### 6.1 问题：设备无法枚举

**可能原因**:
- 缺少 IAD 描述符
- 接口类代码错误
- 端点配置不正确

**解决方案**:
```c
// 确保描述符顺序正确
// 配置 → IAD → 通信接口 → 功能描述符 → 端点 → 数据接口 → 端点
```

### 6.2 问题：主机无法识别为 CDC 设备

**可能原因**:
- bDeviceClass 不是 0x02
- 功能描述符缺失或错误
- 联合描述符接口号不匹配

**解决方案**:
```c
// 验证所有 CDC 特定描述符
// 检查接口号的一致性
// 确保 bmCapabilities 设置正确
```

### 6.3 问题：数据传输丢失或错误

**可能原因**:
- 零长度包处理错误
- 缓冲区管理问题
- 端点 FIFO 配置错误

**解决方案**:
```c
// 正确处理短包和 ZLP
// 验证 USB SRAM 地址分配
// 检查端点中断处理
```

---

## 7. 规范合规性检查清单

### ✓ 设备级检查
- [ ] bDeviceClass = 0x02 (CDC)
- [ ] bDeviceSubClass = 0x00
- [ ] bDeviceProtocol = 0x00
- [ ] bMaxPacketSize0 = 64

### ✓ 描述符检查
- [ ] 包含接口关联描述符 (IAD)
- [ ] 通信接口类代码 = 0x02
- [ ] 数据接口类代码 = 0x0A
- [ ] 包含头功能描述符
- [ ] 包含 ACM 功能描述符
- [ ] 包含联合功能描述符

### ✓ 端点检查
- [ ] 通信接口有1个中断 IN 端点
- [ ] 数据接口有1个批量 OUT 和1个批量 IN 端点
- [ ] 端点大小 ≤ 64 字节 (Full-Speed)
- [ ] 中断端点轮询间隔合理 (1-255ms)

### ✓ 类请求检查
- [ ] 支持 SET_LINE_CODING (0x20)
- [ ] 支持 GET_LINE_CODING (0x21)
- [ ] 支持 SET_CONTROL_LINE_STATE (0x22)
- [ ] 可选支持 SEND_BREAK (0x23)

### ✓ 数据传输检查
- [ ] 正确处理短包
- [ ] 正确发送零长度包
- [ ] 支持任意长度数据传输
- [ ] 流量控制信号处理

### ✓ Embassy 特定检查
- [ ] 驱动配置正确
- [ ] 异步处理实现正确
- [ ] 中断处理符合规范
- [ ] 内存管理无冲突

通过这个详细的规范验证指南，你应该能够确保 Embassy 实现完全符合 USB CDC 标准，解决与主机的兼容性问题。