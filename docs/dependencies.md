# 外部依赖白名单

原则：依赖最小化；新增必须评审并登记于此。rf-core 零外部运行时依赖。

| crate | 用于 | 引入 STEP | 说明 |
|---|---|---|---|
| thiserror | 全部库 crate 错误定义 | 3 | 零运行时开销声明式错误 |
| serde/serde_json | rf-serialization JSON、rf-asset DataAsset/TMX/glTF 解析 | 3 | 行业标准 |
| flate2 | PNG 解压(inflate)、PAK/ZIP 压缩 | 7 | zlib 绑定，纯 Rust 回退可用 |
| windows-sys | rf-platform Win32 窗口/呈现（feature `win32`，仅 windows 目标） | 5 | 零开销 FFI 声明 |

## 待评估（P1/P2，默认不启用）

| crate | 用途 | 优先级 |
|---|---|---|
| libloading | rf-plugin 动态库加载（feature `dynamic`） | P1 |
| zstd / lz4 | PAK 高压缩比 | P1 |
| ash / windows | Vulkan / DX12 真后端（feature `gpu`） | P1 |
| cpal | 音频设备实时输出 | P1 |
| crossbeam-channel | 无锁通道（当前用 std） | P2 |
| notify | 原生文件监听（当前轮询） | P2 |
| Burn/Candle/ort | ML 可插拔后端（feature） | P2 |

## 许可合规

全部为 MIT/Apache-2.0/MIT-OR-Apache 兼容许可。DLSS/XeSS 二进制**不引入**（仅 UpscalerProvider 接口）；
FSR 1/2 为 MIT 可选源码集成（P1，默认不编译）。
