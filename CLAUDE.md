# embassy-ht32f523xx rust lib

Embassy HAL implementation for HT32F523xx microcontrollers

## 🧩 项目结构（当前状态）

```
embassy-ht32/
├── Cargo.toml
├── build.rs
├── memory_ht32f52342.x
├── memory_ht32f52352.x
├── src/
│   ├── lib.rs              # 库入口点
│   ├── chip/               # 芯片特定定义
│   │   ├── ht32f52342.rs
│   │   ├── ht32f52352.rs
│   │   └── mod.rs
│   ├── gpio.rs             # GPIO 驱动
│   ├── rcc.rs              # 时钟和复位控制
│   ├── time.rs             # 时间单位定义
│   ├── time_driver.rs      # Embassy 时间驱动
│   ├── timer.rs            # 定时器和PWM驱动
│   ├── uart.rs             # UART 驱动
│   ├── usb.rs              # USB 驱动
│   ├── flash.rs            # 闪存驱动
│   ├── exti.rs             # 外部中断
│   ├── interrupt.rs        # 中断处理
│   └── fmt.rs              # 格式化工具
└── bsp/                    # 板级支持包
    ├── Cargo.toml
    ├── src/lib.rs
    └── src/esk32_30501.rs
```

## 📊 实现状态

### ✅ 完整实现
| 模块           | 状态 | 说明                                    |
| ------------ | -- | ------------------------------------- |
| `time.rs`    | ✅  | 完整的时间单位定义（Hertz, Microseconds）       |
| `gpio.rs`    | ✅  | 完整的GPIO实现，支持多种模式                      |
| `flash.rs`   | ✅  | 完整的NorFlash trait实现                   |
| `rcc.rs`     | ✅  | 时钟和复位配置                               |
| `timer.rs`   | ✅  | 基础定时器和PWM实现                           |

### ⚠️ 部分实现
| 模块               | 状态 | 说明                          |
| ---------------- | -- | --------------------------- |
| `usb.rs`         | ⚠️  | USB驱动基础实现，需要更多硬件特定配置       |
| `uart.rs`        | ⚠️  | 基础UART支持，Embassy异步trait待完善 |
| `interrupt.rs`   | ⚠️  | 基础中断处理结构，处理程序待完善          |
| `exti.rs`        | ⚠️  | 简化的外部中断实现                   |
| `time_driver.rs` | ⚠️  | Embassy时间驱动基础实现             |

### ❌ 缺失功能
- I2C 驱动
- I2S 驱动
- ADC 驱动
- DMA 支持
- 完整的异步trait实现
- 高级PWM功能


## 依赖库信息

### ht32f523x2 Peripheral access API

* ht32f523x2 rust Peripheral access API placed under ./deps/ht32f523x2/
* svd file placed udner ./deps/ht32f523x2/HT32F52342_52.svd
