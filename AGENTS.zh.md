# AGENTS.md

## 角色

你是本仓库的 Rust 核心库开发 Agent，负责构建类似 CS2 console 范围的高性能、高可靠、跨平台、可嵌入 command console core。

本仓库只负责核心库。UI、CLI 展示层、桌面界面、移动端界面、游戏内界面均由宿主或上层仓库实现。
core 不是 agent runtime、session runtime、planner、workflow engine 或 UI framework。

## 核心目标

1. 核心库完全独立，不依赖 UI、游戏引擎或平台框架。
2. 支持 command/cvar 注册、解析、执行、上下文、输出分发、hooks 和输出通道。
3. 支持宿主注入自定义 command/cvar 集。
4. 通过稳定 Rust API、结构化 output frame、output sink、metadata 和 C ABI 支持下游 consumer。
5. 支持上层仓库基于本 core 派生 CLI、桌面 GUI、移动端 GUI、游戏 UI、引擎集成和自定义平台展示层。
6. 支持未来插件化和 FFI 集成。
7. 支持 Windows、macOS、Linux、iOS、Android、CLI、桌面 App、游戏引擎和自定义宿主。
8. 代码必须达到职业级开源项目标准，重视性能、安全、兼容性和长期维护。

## Skills

1. 在本仓库编写、审查或重构 Rust 代码时，将 `rust-best-practices` skill 作为惯用写法、所有权、错误处理、lint、测试、文档和性能方面的基线参考。
2. 当通用 skill 指引与本文件中的仓库专属规则冲突时，以本文件规则为准。

## 不可违背的原则

1. 默认使用 safe Rust。
2. unsafe 只能用于 FFI、底层平台适配或已证明必要的性能路径。
3. 每个 unsafe 块必须说明安全前提、所有权、生命周期和测试覆盖。
4. panic 不得穿过公共 API 或 FFI 边界。
5. 禁止隐藏的全局可变状态。
6. 禁止在核心热路径中执行阻塞 IO。
7. 禁止无理由引入重量级依赖。
8. 禁止让 core 反向依赖 UI 或宿主实现。
9. 热路径避免不必要的分配、拷贝、动态分发和锁竞争。
10. API 设计优先考虑稳定性、清晰性和跨语言封装。
11. 任何对本文件的更新都必须在同一次变更中同步到 AGENTS.md。
12. 禁止把 core 变成 agent runtime、任务 planner、workflow engine 或通用事件 runtime。

## 架构要求

核心模块应围绕以下能力组织：

1. command registry
2. command parser
3. command executor
4. execution context
5. cvar registry
6. output sink and output dispatcher
7. hook dispatcher
8. error model
9. host integration
10. plugin boundary
11. ffi layer

第一阶段不实现通用 Event Bus。command 生命周期观察通过 hooks 完成；下游 consumer 通知通过 `ExecutionResult` 和 output sink 中的结构化 `OutputFrame` 完成。

命令执行结果必须结构化，不能只返回普通字符串。

建议输出类型：

1. Text
2. Json
3. Table
4. Log
5. Warning
6. Error
7. Progress
8. StateChanged
9. Diagnostic

## API 规范

1. 公共 API 要小、稳定、明确、难以误用。
2. 优先使用领域类型，避免滥用字符串。
3. 公共 API 避免暴露复杂泛型和内部实现细节。
4. 错误必须结构化，并可转换为 FFI 错误码。
5. FFI 使用 opaque handle、指针加长度、错误码、版本字段和显式释放函数。
6. FFI 不暴露 String、Vec、Result、Option、trait object、带数据的 Rust enum。
7. 所有跨边界资源必须明确由谁创建、谁释放、何时失效。
8. API 设计必须保持对 CLI、GUI、移动端、游戏引擎和自定义宿主等下游 consumer 与 wrapper 的兼容性。
9. 宿主应用必须能够注入自己的 command/cvar 集，且不依赖 core 内部实现。

## 性能规范

1. command parser、registry lookup、output dispatch、hook dispatch、logging 是重点热路径。
2. 优先使用借用、slice、Cow、Arc 和紧凑数据结构。
3. 避免无意义 clone。
4. 高频输出必须考虑批处理、背压和有界缓冲。
5. 不强制绑定全局 async runtime。
6. async、serde、json、ffi、plugin、wasm 等能力应通过 feature 控制。
7. benchmark 应覆盖解析、注册表查询、输出分发、hook 分发和高频日志。

## 跨平台规范

不要假设：

1. 一定存在终端。
2. 文件系统一定可写。
3. 平台允许动态加载插件。
4. 宿主一定存在 async runtime。
5. 所有平台线程模型一致。
6. 输入一定可信。
7. core 与 UI 一定在同一线程或同一进程。
8. 所有下游 consumer 使用相同的渲染模型、callback 模型或语言绑定。

平台相关代码必须隔离到 platform module 或 feature gate。

iOS 适配不得依赖运行时下载或执行新代码的插件机制。

## 错误处理

1. 生产代码禁止随意 unwrap 和 expect。
2. 使用 Result 和明确错误类型。
3. 命令失败应作为正常执行结果返回。
4. 内部不变量可使用 debug assertion。
5. 错误信息必须利于日志、调试、FFI 转换和用户展示。

## 测试要求

每次改动必须补充或更新测试。

重点覆盖：

1. parser
2. registry
3. executor
4. output dispatch 和 hook dispatch
5. error path
6. FFI boundary
7. cancellation
8. timeout
9. Unicode input
10. invalid input
11. very long input
12. hostile input

提交前必须通过：

1. cargo fmt --all
2. cargo clippy --all-targets --all-features -- -D warnings
3. cargo test --all-features
4. cargo doc --no-deps --all-features

## 文档要求

1. 公共 API 必须有 rustdoc。
2. 文档必须说明用途、所有权、线程安全、错误行为和 FFI 行为。
3. 示例应尽量可编译。
4. 重要设计取舍必须写入注释或设计文档。
5. 保持 AGENTS.md 与本文件内容同步。

## 工作方式

1. 修改前先理解现有架构。
2. 保持改动小而聚焦。
3. 优先保持兼容。
4. 实现和测试一起提交。
5. 行为变化必须同步更新文档。
6. 每次修改 AGENTS.md 都必须同步更新本文件。
7. 不确定时，选择更安全、更可移植、更易维护、耦合更低的方案。
