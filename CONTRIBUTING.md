# 贡献指南 / Contributing

感谢你考虑为 **zhmm-tauri** 贡献代码！本指南帮助你快速上手。
*English speakers: the sections below are concise enough to read with your browser's translator. PRs welcome in either 中文 or English.*

---

## 🐛 报告 Bug

1. 先在 [Issues](https://github.com/szgenle/zhmm-tauri/issues) 搜索是否已有相同问题
2. 若无，新建 Issue，清楚给出：
   - 复现步骤、期望 vs 实际
   - 操作系统及版本（macOS / Windows / Linux 发行版）
   - 应用版本（关于页面或 `package.json` / `Cargo.toml` 中的 version）
   - WebView 版本（macOS WKWebView / Windows WebView2 / Linux WebKitGTK）
   - 相关日志与截图（**请先脱敏**）

> ⚠️ **不要**在 Issue 中贴真实账号 / 密码 / 云凭据 / TOTP Secret。
> ⚠️ 若涉及安全漏洞，请改走 [SECURITY.md](SECURITY.md) 的私下披露流程。

## 💡 提交功能建议

新建 Issue 重点描述 *用户场景* 而非具体实现，便于讨论替代方案。

## 🧑‍💻 提交代码（Pull Request）

### 环境准备

- [Node.js](https://nodejs.org/) ≥ 18（建议使用 LTS）
- [Rust](https://www.rust-lang.org/tools/install) ≥ 1.77，含 `cargo`、`rustfmt`、`clippy`
- 平台依赖详见 [Tauri 官方先决条件](https://tauri.app/start/prerequisites/)

```bash
git clone https://github.com/szgenle/zhmm-tauri.git
cd zhmm-tauri
make install               # npm install + cargo fetch
make dev                   # 启动开发模式验证环境就绪
```

### 工作流

1. Fork 本仓库，在你的 fork 上基于 `main` 切分支：`git checkout -b feat/my-change`
2. 提交前本地自检：
   ```bash
   make fmt          # cargo fmt
   make lint         # vue-tsc --noEmit + cargo clippy -- -D warnings
   make test         # cargo test
   ```
3. Commit 信息建议遵循 [Conventional Commits](https://www.conventionalcommits.org/)：
   - `feat: 新增 xxx`
   - `fix: 修复 xxx`
   - `docs: 更新文档`
   - `refactor: 重构 xxx`
   - `test: 补充测试`
   - `chore: 其它杂项`
4. 推到你的 fork，针对上游 `main` 发起 PR
5. 耐心等待 Review，根据反馈修改

### 代码规范

#### 前端（Vue 3 + TypeScript）

- 使用 `<script setup lang="ts">` SFC 风格
- 优先使用 Composition API，复杂逻辑抽到 `src/composables/`
- 业务调用 Tauri 后端统一走 `src/api.ts` 封装，不要在组件内直接 `invoke`
- 命名：组件 PascalCase（`PasswordEditDialog.vue`），composable `useXxx`

#### 后端（Rust）

- 遵循 `cargo fmt` 默认格式（行宽 100，4 空格缩进）
- 提交前必须 `cargo clippy -- -D warnings` 全绿
- 公共 API 加 `///` 文档注释；安全相关代码必须含设计意图说明
- 涉及加密 / 密钥处理的改动：
  - 必须使用 `zeroize` 清理敏感缓冲
  - 必须使用 `subtle::ConstantTimeEq` 进行常时比较
  - PR 描述明确说明威胁模型变化
- Tauri command 加在 [src-tauri/src/commands.rs](src-tauri/src/commands.rs)，并在 `lib.rs` 的 `invoke_handler!` 中注册；同时更新前端 `api.ts` 类型签名

### 测试

- Rust 单元测试位于各模块的 `#[cfg(test)] mod tests`：
  ```bash
  cd src-tauri && cargo test
  cd src-tauri && cargo test --release       # 性能敏感场景
  ```
- 涉及加密 / 文件格式 / 互通性的改动**强烈建议**：
  - 提供加密往返、错密码、错账号、字节篡改的测试用例（参考 [crypto.rs](src-tauri/src/crypto.rs) 现有测试）
  - 若改动涉及与 Python 版的互通，请在 PR 描述中给出双向打开同一文件的验证步骤

### 与 Python 版 `zhmm` 的互通约束

本项目与 [Python 版 zhmm](https://github.com/szgenle/zhmm) **共享 `.zmb` 二进制格式**。涉及以下任一改动均视为破坏性变更，需与 Python 版同步：

- `magic` / `version` 字段
- Argon2 默认参数与文件头布局
- SM4-GCM 的 IV / tag 长度、AAD 范围
- TOTP 私有 SM3 算法名

PR 描述中需明确标注「⚠️ 影响 Python 版互通」并讨论迁移策略。

### 项目目录结构

```
zhmm-tauri/
├── src/                       # 前端：Vue 3 + TypeScript
├── src-tauri/src/             # 后端：Rust
│   ├── crypto.rs              # 加密层（v6 SM4-GCM / v5 兼容读）
│   ├── vault.rs               # 密码库状态、解锁、备份、rekey
│   ├── accounts.rs            # 多账号库与最近文件索引
│   ├── commands.rs            # Tauri command 入口
│   ├── models.rs              # 数据模型
│   ├── io_json.rs / io_xlsx.rs# 持久化与导入导出
│   ├── totp.rs                # RFC 6238 + SM3 扩展
│   ├── site_catalog.rs        # 离线网站词典
│   ├── settings.rs            # 应用配置
│   └── anti_capture.rs        # 防截屏（macOS / Windows）
├── resources/site_catalog.json
├── Makefile
└── README.md / SECURITY.md / LICENSE / ...
```

## 📜 许可与所有权

你同意你的贡献在 [MIT License](LICENSE) 下发布。你对自己的提交拥有完整版权或授权，且不包含任何他人作品未经授权的部分。

## 💬 行为准则

参与本项目即表示遵守 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)。请保持友善、建设性、可包容。

---

再次感谢你的贡献！🎉
