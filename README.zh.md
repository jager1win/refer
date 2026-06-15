### Refer - 用户参考手册构建器

- - -

主要目的是为那些不仅需要存储数据，还需要在“野外”没有互联网连接的情况下即时进行计算的人提供一个快速的本地工具。

- - -

### 核心功能 — 运算
可选添加。将您的参考手册转变为工程计算工具。

使用变量创建公式，变量可以是：
- 来自参考手册本身的 `number` 类型字段，
- 手动输入的值，
- 或两者同时使用。

每个运算都应用于参考手册中的所有项目。

使用 [exmex](https://docs.rs/exmex/latest/exmex/index.html) 库实现 — 有关变量、运算符和常量的语法，请参阅其文档。

- - -

### 版本
Android、Linux、Windows — 所有构建版本均在 Github 上。

### 语言
13种语言。EN 和 RU 为手动翻译，其余由 LLM 生成：ES、FR、DE、PT、ZH、JA、KO、IT、NL、TR、AR。

- - -

### 内置示例
6个带有预定义运算的示例参考手册。随意修改和编辑它们。只需点击“创建”菜单中的一次即可重新创建。

- **Shrinkflation** — 比较单位重量/体积的价格
- **Dilution** — 计算溶液混合比例
- **Ballistics** — 弹道轨迹计算器
- **Deposit** — 计算复利增长
- **Geometry** — 圆和球体的尺寸，输入半径
- **Oscillator** — 时间 t 处的波值 — 使用时间提示作为参考

- - -

### 存储
目录 `~/Documents/refer`，扩展名 `.refer`。这些实际上是 SQLite 数据库。在一个设备上创建，转移到另一个设备。

支持子文件夹 — 内置示例创建在 `refer/example` 中。

应用程序仅与此文件夹交互，完全不使用互联网。

- - -

### 导入
从零开始创建参考手册或从以下格式导入：

- **CSV、TSV** — 首选 UTF-8 编码
- **XLS、XLSX、ODT** — 导入第一个工作表。不推荐用于非常大的文件：导入慢，结果文件大
- **SQLite** — 导入第一个表

- - -

### 原则

**简洁性。** 所有已知的替代方案都功能过载。Refer 针对特定项目工作；从不显示完整列表 — 搜索最多返回 10 个项目。

**最小限制。** 您可能会遇到错误 — 在“设置”菜单中或主屏幕上显示的路径处查看日志。

**安全性。** 故意不添加加密 — 请使用您的操作系统工具。

- - -

### ⚠ Android 警告
需要“访问所有文件”权限才能使用 `Documents/refer` 文件夹。

手动启用（取决于您的手机）：
- 设置 → 应用 → Refer → 权限 → 所有权限
- 特殊访问权限 → 所有文件访问权限 → 启用

- - -

[Rust](https://github.com/rust-lang/rust) · [Tauri](https://github.com/tauri-apps/tauri) · [Leptos](https://github.com/leptos-rs/leptos)  
运算 — [Exmex](https://docs.rs/exmex/latest/exmex/index.html)

开源 (GPL-3.0)。无广告、无数据收集、无云服务。