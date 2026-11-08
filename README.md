# DeepSeek Harness Tauri 桌面版

非官方 Linux 桌面封装。Tauri 启动官方 `@deepseek-ai/dsh` 本地 Web 服务，仅监听 127.0.0.1 的随机端口，退出时结束服务。启动后需要按 Harness 的提示配置账号或 API 密钥。

本项目固定使用 `@deepseek-ai/dsh` 0.2.2-rc.1；它要求 Node.js 24.9 或更新版本。开发构建需要 Rust、Tauri 2 的 Linux 开发依赖（GTK 3、WebKitGTK 4.1、AppIndicator、librsvg、patchelf）和 Node.js 24。项目的 `src-tauri/resources/runtime` 是构建资源，包含本地运行时及 Harness 依赖，不属于应用自己的源码。

```bash
export PATH="$PWD/src-tauri/resources/runtime/node/bin:$PATH"
npm ci
npm run build
```

产物位于 `src-tauri/target/release/bundle/deb/`。更换 Node 或 Harness 版本时需同步更新运行时、锁文件并验证服务。此封装不代表 DeepSeek 官方出品。
