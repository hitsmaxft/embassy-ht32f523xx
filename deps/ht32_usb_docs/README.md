# HT32F52352 USB协议栈文档包

## 📖 文档概述

本文档包提供了从ChibiOS主干代码中提取的HT32F52352微控制器USB协议栈的完整技术文档，包括寄存器定义、初始化流程、中断处理、数据传输和配置示例。所有代码片段和注释均来自实际的ChibiOS源代码，可直接用于项目开发。

**文档总计**: 6个Markdown文件，3685行内容  
**代码来源**: ChibiOS-Contrib (GitHub)  
**目标芯片**: HT32F52352 (Cortex-M0+, USB 2.0 Full-Speed)

---

## 📚 文档列表

| 序号 | 文档名称 | 行数 | 大小 | 描述 |
|------|----------|------|------|------|
| 0 | [00_完整报告.md](./00_完整报告.md) | 551 | 15KB | **★主报告★** 精简汇总，快速索引 |
| 1 | [01_USB寄存器定义.md](./01_USB寄存器定义.md) | 283 | 9KB | USB寄存器地址、位定义和操作 |
| 2 | [02_USB初始化流程.md](./02_USB初始化流程.md) | 527 | 14KB | USB驱动和外设初始化详解 |
| 3 | [03_USB中断处理.md](./03_USB中断处理.md) | 777 | 19KB | USB中断服务程序和事件处理 |
| 4 | [04_USB数据传输.md](./04_USB数据传输.md) | 845 | 20KB | IN/OUT端点数据传输详解 |
| 5 | [05_USB配置示例.md](./05_USB配置示例.md) | 702 | 20KB | 完整的配置文件和代码示例 |
| 6 | [06_USB_CDC_详细分析.md](./06_USB_CDC_详细分析.md) | 850+ | 25KB | **★Embassy CDC★** USB CDC 引导序列和 Embassy 实现指南 |
| 7 | [07_USB_CDC_寄存器时序图.md](./07_USB_CDC_寄存器时序图.md) | 400+ | 12KB | **★时序图★** 完整的 CDC 枚举寄存器时序和调试检查点 |
| 8 | [08_USB_CDC_规范验证.md](./08_USB_CDC_规范验证.md) | 650+ | 20KB | **★规范验证★** USB CDC 标准合规性检查和测试指南 |

---

## 🚀 快速开始

### 新手入门路径
```
1. 阅读 00_完整报告.md (快速了解整体架构)
   ├─ USB硬件特性
   ├─ 核心代码位置
   └─ 关键配置参数

2. 学习 01_USB寄存器定义.md (理解硬件接口)
   ├─ 寄存器基址和结构
   ├─ 控制和状态寄存器
   └─ 端点寄存器操作

3. 实践 02_USB初始化流程.md (启动USB)
   ├─ 时钟配置
   ├─ 驱动初始化
   └─ 端点配置

4. 理解 03_USB中断处理.md (处理USB事件)
   ├─ 中断服务程序
   ├─ EP0控制传输
   └─ 数据端点中断

5. 掌握 04_USB数据传输.md (收发数据)
   ├─ OUT传输(接收)
   ├─ IN传输(发送)
   └─ 多包传输

6. 配置 05_USB配置示例.md (项目配置)
   ├─ mcuconf.h配置
   ├─ halconf.h配置
   └─ USB描述符
```

### Embassy/CDC 专门路径 ⭐
```
1. 先完成基础学习 (上述步骤1-6)

2. 深入 06_USB_CDC_详细分析.md (Embassy CDC实现)
   ├─ CDC 枚举序列详解
   ├─ 数据流分析
   ├─ Embassy 实现要点
   └─ 与 ChibiOS 差异对比

3. 参考 07_USB_CDC_寄存器时序图.md (调试必备)
   ├─ 完整枚举时序图
   ├─ 关键寄存器状态检查点
   ├─ 调试检查清单
   └─ 常见问题排查

4. 验证 08_USB_CDC_规范验证.md (合规性检查)
   ├─ USB CDC 标准要求
   ├─ 描述符规范验证
   ├─ 类请求处理验证
   └─ Embassy 特定检查
```

### 有经验开发者
直接查阅具体章节：
- **寄存器速查**: 01_USB寄存器定义.md
- **代码片段**: 02-05各文档的代码示例
- **调试问题**: 00_完整报告.md → 常见问题速查

---

## 📂 代码文件位置参考

### ChibiOS-Contrib源代码
```
核心驱动:
  ChibiOS-Contrib/os/hal/ports/HT32/LLD/USBv1/
  ├── hal_usb_lld.h         # USB驱动头文件
  └── hal_usb_lld.c         # USB驱动实现 (712行)

寄存器定义:
  ChibiOS-Contrib/os/common/ext/CMSIS/HT32/HT32F523xx/
  ├── ht32f523x2.h          # 芯片定义
  └── ht32f523x2_reg.h      # 寄存器定义 (688行)

示例代码:
  ChibiOS-Contrib/demos/HT32/HT32F165x_USB_DFU/
  ├── main.c                # 主程序
  ├── source/usbdfu.c       # DFU实现
  └── cfg/
      ├── mcuconf.h         # MCU配置
      └── halconf.h         # HAL配置
```

---

## 🔑 核心技术要点

### USB硬件特性
- **USB版本**: USB 2.0 Full-Speed (12 Mbps)
- **端点数量**: 8个 (EP0-EP7)
- **USB SRAM**: 1024字节
- **支持传输**: 控制、批量、中断、同步

### 关键配置
```c
// USB时钟配置 (必须为48MHz)
#define HT32_USB_PRESCALER  3    // 144MHz / 3 = 48MHz

// USB驱动使能
#define HT32_USB_USE_USB0   TRUE

// 中断优先级
#define HT32_USB_USB0_IRQ_PRIORITY  5
```

### 寄存器基址
```c
#define USB_BASE        0x400A8000
#define USB_SRAM_BASE   0x400AA000
```

---

## 📊 文档使用指南

### 按主题查找

#### 寄存器操作
- **文档**: 01_USB寄存器定义.md
- **内容**: USB->CSR, USB->IER, USB->EP[n].*
- **适用**: 直接寄存器操作，底层调试

#### 初始化配置
- **文档**: 02_USB初始化流程.md + 05_USB配置示例.md
- **内容**: 时钟配置，驱动启动，端点初始化
- **适用**: 项目启动阶段

#### 中断处理
- **文档**: 03_USB中断处理.md
- **内容**: USB_IRQHandler, SOF/RESET/SUSPEND等
- **适用**: 事件处理，协议交互

#### 数据传输
- **文档**: 04_USB数据传输.md
- **内容**: IN/OUT传输，USB SRAM操作
- **适用**: 数据收发实现

#### 完整示例
- **文档**: 05_USB配置示例.md
- **内容**: 描述符，回调函数，main.c
- **适用**: 快速搭建项目

---

## 🛠️ 实用代码片段索引

### 基础操作
```c
// 连接USB
usb_lld_connect_bus(&USBD1);      // USB->CSR |= DPPUEN

// 断开USB
usb_lld_disconnect_bus(&USBD1);   // USB->CSR &= ~DPPUEN

// 获取帧号
uint16_t frame = usb_lld_get_frame_number(&USBD1);
```

### 数据传输
```c
// 发送数据
usbStartTransmitI(&USBD1, ep, data, size);

// 接收数据
usbStartReceiveI(&USBD1, ep, buffer, size);

// 获取接收大小
size_t n = usb_lld_get_transaction_size(&USBD1, ep);
```

### 端点控制
```c
// STALL端点
usb_lld_stall_in(&USBD1, ep);
usb_lld_stall_out(&USBD1, ep);

// 清除STALL
usb_lld_clear_in(&USBD1, ep);
usb_lld_clear_out(&USBD1, ep);
```

---

## 🐛 常见问题快速查找

| 问题 | 查看文档 | 章节 |
|------|----------|------|
| USB无法枚举 | 00_完整报告.md | 常见问题速查 → 问题1 |
| 数据传输失败 | 04_USB数据传输.md | 第9节 常见问题 |
| 中断不触发 | 03_USB中断处理.md | 第10节 问题排查 |
| 时钟配置错误 | 02_USB初始化流程.md | 第9节 常见问题 |
| 端点内存溢出 | 02_USB初始化流程.md | 第9节 问题2 |
| 寄存器操作错误 | 01_USB寄存器定义.md | 各寄存器操作示例 |

---

## 🎯 学习建议

### 理论学习
1. **USB协议基础**: 建议先了解USB 2.0规范基础
2. **端点概念**: 理解控制端点、批量端点等
3. **传输类型**: OUT传输(接收)、IN传输(发送)

### 实践步骤
1. **搭建环境**: 准备HT32F52352开发板和USB线
2. **编译示例**: 编译DFU示例代码
3. **修改配置**: 修改VID/PID和描述符
4. **测试传输**: 实现简单的数据收发
5. **添加功能**: 实现自己的USB设备类

### 调试技巧
1. **使用USB分析仪**: 查看实际USB通信
2. **添加调试输出**: 在关键位置打印日志
3. **LED指示**: 使用LED显示USB状态
4. **单步调试**: 在中断中设置断点

---

## 📖 延伸阅读

### USB规范文档
- USB 2.0 Specification
- USB Device Class Specifications (CDC, MSC, HID等)

### ChibiOS文档
- ChibiOS HAL Documentation
- ChibiOS USB Driver Guide
- ChibiOS Forum: http://forum.chibios.org

### HT32芯片文档
- HT32F52342/52352 Datasheet
- HT32F52352 User Manual
- Holtek官方支持: https://www.holtek.com

---

## 📝 版本信息

| 项目 | 信息 |
|------|------|
| **文档版本** | 1.0 |
| **创建日期** | 2025年11月 |
| **作者** | MiniMax Agent |
| **代码来源** | ChibiOS-Contrib (GitHub) |
| **目标芯片** | HT32F52352 |
| **许可证** | 基于Apache License 2.0 |

---

## 💡 使用提示

### 文档标记说明
- **★**: 重点内容
- **✓**: 正确示例
- **❌**: 错误示例
- **⚠️**: 注意事项
- **📌**: 重要提示

### 代码块说明
- 带有完整注释的代码可直接使用
- 示例代码已经过验证
- 配置参数可根据实际需求调整

### 搜索技巧
- 使用Ctrl+F在文档内搜索关键字
- 所有寄存器名称都可搜索定位
- 函数名称可快速定位到实现代码

---

## 🤝 反馈和改进

如有问题或建议，请参考:
- ChibiOS论坛: http://forum.chibios.org
- ChibiOS GitHub: https://github.com/ChibiOS/ChibiOS-Contrib

---

## ✅ 快速检查清单

开始USB开发前的检查清单：

- [ ] 已阅读00_完整报告.md
- [ ] 理解USB硬件特性
- [ ] 掌握寄存器基本操作
- [ ] 了解初始化流程
- [ ] 理解中断处理机制
- [ ] 掌握数据传输方法
- [ ] 准备好配置文件
- [ ] 准备好调试工具

---

**文档说明完成**

> 📌 **开始使用**: 建议从 [00_完整报告.md](./00_完整报告.md) 开始阅读，获取整体概览，然后根据需要查阅具体的技术文档。祝开发顺利！
