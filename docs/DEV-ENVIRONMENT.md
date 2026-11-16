# 开发环境验证记录

2026-11-11 商品 JSON 上传与用户中心统计交付已在目标 Linux 环境以默认并行方式完整通过 **46/46** 项测试，零忽略；生产数据库与服务未参与隔离测试。Windows 本机存在以下已复现的测试进程异常，底层 native 根因尚未定位。

- 本机 Rust 1.96.0 / MSVC 14.44.35207：完整 46 项默认并行执行时，进程退出 `0xc0000005 STATUS_ACCESS_VIOLATION`，没有 Rust 断言失败。Windows 应用错误记录的模块为 unknown、异常偏移为 `0x1`。
- 相同源码在独立的全新 target 目录重建仍复现；已安装的 Rust 1.96.1 配合另一独立 target 目录也复现，未调整测试并发。
- 排除商品和统计模块的旧 36 项组合仍复现。商品、统计、管理员账号、应用配置、业务授权、阶梯奖励、红包雨、邮件及用户调查等模块分别运行均通过。
- 上轮 0.3.7 商品版归档（不含本轮 JSON、arbitrary_precision 或 user_stats 改动）在隔离目录运行其完整 **39 项**默认并行测试，同样出现上述异常。因此已有旧基线能复现，不能将异常归因于本轮商品 JSON 或统计改动。
- 旧基线首次调试构建另遇 `LNK4209/LNK1103`：qrcode/chrono 对象的 CodeView 调试信息损坏。仅在该隔离基线禁用调试符号后成功链接，其 39 项测试仍出现相同 native 异常；源码、测试内容和并发均保持原样。

当前源码的复现命令：

```powershell
cargo test --offline --manifest-path server/Cargo.toml
cargo test --offline --manifest-path server/Cargo.toml --target-dir server/target-verification-clean
cargo +1.96.1 test --offline --manifest-path server/Cargo.toml --target-dir server/target-verification-msvc1961
```

旧基线来源：`D:/工作/services/lingzi-mail/output/orange-admin-mail.tar.gz`。仅提取其中 server 至项目的 `private/verification/baseline-products-20261111`，与当前源码隔离。旧基线链接对照命令如下，环境变量只用于该 PowerShell 进程：

```powershell
$env:CARGO_PROFILE_TEST_DEBUG='0'
$env:CARGO_PROFILE_DEV_DEBUG='0'
cargo test --offline --manifest-path private/verification/baseline-products-20261111/server/Cargo.toml
```

目标 Linux 的完整套通过，Windows `cargo check` 也通过且无警告。以上 Windows 结果作为开发环境问题保留；当前交付采用目标 Linux 完整套的实际结果，未通过串行化、加锁或忽略测试处理该异常。
