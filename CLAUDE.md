# HT32 Embassy Project Structure

一个 带 Embassy 依赖 且 支持 probe-rs 的 HT32F523xx 工作区骨架。

是完整可直接放进 Git 仓库的初始版本，本地 cargo run 就能用 probe-rs 烧到板子（有 CMSIS-DAP/J-Link/ST-Link 等兼容调试器）。

目标 MCU 是 HT32F523xx（例如 HT32F52352）。
已有 Rust PAC 支持、官方资料与开发板可以拿来做测试。下面是我刚做的现实性检查、结论与建议的下一步执行计划（可马上开始）。

我刚查到的关键事实（可直接用来开工）

    有社区维护的 HT32 PAC 项目 ht32-rs，为 HT32 系列提供 svd2rust 生成的 PACs。
    GitHub

    在 crates.io 上已有针对 HT32F523x2（覆盖 HT32F52352 / 52342 等）的 PAC crate ht32f523x2，可以直接依赖。
    Crates.io

    Holtek 官方页面与 datasheet 明确 HT32F523xx 是 Cortex-M0+、最高 48MHz，并且该系列包含 USB、PDMA（DMA）、多种定时器等外设（我们会在 HAL/embassy 实现中用到这些特性）。
    Holtek+1

    有官方/第三方的 starter kit / dev board（例如 ESK32-30501），用于上板测试和调试会很方便。
    Best Modules

## project plan

embassy-ht32/
├─ ht32-pac/         # optional: 或依赖 crates.io 的 pac
├─ ht32-hal/         # blocking HAL（GPIO, RCC, Timer, UART）
├─ embassy-ht32/     # async wrappers + embassy trait impls
├─ bsp/              # board support: pin mappings, features
└─ examples/
   ├─ blink-embassy/
   └─ serial-echo/
搭建 workspace（推荐优先） — 我会 scaffold 一个 embassy-ht32 workspace，包含：

    直接依赖 ht32f523x2（crates.io）作为 ht32-pac。
    Crates.io

    ht32-hal（blocking 基础：GPIO、RCC/Clocks、Timer、UART 最小实现）

    embassy-ht32（async wrappers：优先实现 Timer 与 UART）

    bsp（若需，针对 ESK32-30501 或你手头开发板的引脚定义）

    examples/blink-embassy 与 examples/serial-echo（可用 probe-rs / probe-run 上载）
    我会把 README、Cargo.toml、CI skeleton（仅编译为主）也一并做好。

验证 PAC / 编译测试 — 在 scaffold 完成后，我会编译 no_std blink demo（仅交叉编译）确认 PAC crate 与依赖无编译错误；如果发现 SVD 里的寄存器/位域缺失，会列出具体 patch 建议。
GitHub
Crates.io

在目标板上跑通最小例（需要你或我有物理板与 probe） — 把 blink / UART echo 烧到 ESK32-30501 或你的板上，确认时钟、GPIO、UART 工作正常。若你没有板，我仍可把能直接复用的代码推到仓库，你拿板执行即可。
Best Modules+1

实现 Embassy 核心（后续） — 当基础 HAL 成熟后，继续实现 embassy-wrappers：embassy-time（基于 SysTick 或 GPTM）、中断驱动的 async UART（AtomicWaker 机制）以及后续的 SPI/I2C/DMA 支持。


## development

# 安装目标和 probe-rs
rustup target add thumbv6m-none-eabi
cargo install probe-rs --locked
cargo install cargo-embed --locked

# 编译
cargo build --release

# 烧录 & 运行
cargo run --release -p blink-embassy

## rmk intergation

