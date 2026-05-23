.PHONY: help install dev dev-fe build build-debug clean clean-fe clean-rs clean-all fmt fmt-rs lint lint-rs test check env-info cli cli-install cli-uninstall cli-help

# CLI 安装位置（可覆盖：make cli-install CLI_PREFIX=/usr/local）
CLI_PREFIX ?= $(HOME)/.local
CLI_BIN_DIR := $(CLI_PREFIX)/bin
CLI_BIN := $(CLI_BIN_DIR)/zhmm-cli

# 默认目标：显示帮助信息
help:
	@echo "可用的 Make 命令："
	@echo ""
	@echo "  make install       - 安装所有依赖（npm + cargo）"
	@echo "  make dev           - 启动 Tauri 开发模式（前端 + 后端热更新）"
	@echo "  make dev-fe        - 仅启动前端开发服务器（不启动 Tauri）"
	@echo "  make build         - 构建生产版本应用程序"
	@echo "  make build-debug   - 构建调试版本应用程序"
	@echo "  make clean         - 清理所有构建产物"
	@echo "  make clean-fe      - 清理前端构建产物"
	@echo "  make clean-rs      - 清理 Rust 构建产物"
	@echo "  make fmt           - 格式化所有代码（前端 + Rust）"
	@echo "  make fmt-rs        - 仅格式化 Rust 代码"
	@echo "  make lint          - 代码检查（TypeScript + Rust）"
	@echo "  make lint-rs       - 仅检查 Rust 代码（clippy）"
	@echo "  make test          - 运行 Rust 单元测试"
	@echo "  make check         - Rust 编译检查（不生成产物，速度快）"
	@echo "  make env-info      - 显示开发环境信息"
	@echo "  ----- CLI 模式 -----"
	@echo "  make cli           - 构建 zhmm-cli release 二进制"
	@echo "  make cli-install   - 构建并安装到 ~/.local/bin（可改 CLI_PREFIX）"
	@echo "  make cli-uninstall - 卸载 zhmm-cli"
	@echo "  make cli-help      - 打印 zhmm-cli 速查卡（不用记子命令）"


# 安装所有依赖
install:
	@echo "安装前端依赖..."
	npm install
	@echo "检查 Rust 依赖..."
	cd src-tauri && cargo fetch
	@echo "所有依赖安装完成！"

# 启动 Tauri 开发模式
dev:
	@echo "启动 Tauri 开发模式..."
	npm run tauri dev

# 仅启动前端开发服务器
dev-fe:
	@echo "启动前端开发服务器..."
	npm run dev

# 构建生产版本
build:
	@echo "构建生产版本..."
	npm run tauri build
	@echo "构建完成！产物位于 src-tauri/target/release/bundle/"

# 构建调试版本
build-debug:
	@echo "构建调试版本..."
	npm run tauri build -- --debug
	@echo "调试版本构建完成！产物位于 src-tauri/target/debug/"

# 清理前端构建产物
clean-fe:
	@echo "清理前端构建产物..."
	rm -rf dist

# 清理 Rust 构建产物
clean-rs:
	@echo "清理 Rust 构建产物..."
	cd src-tauri && cargo clean

# 清理所有构建产物
clean: clean-fe clean-rs
	@echo "所有构建产物已清理！"

# 格式化 Rust 代码
fmt-rs:
	@echo "格式化 Rust 代码..."
	cd src-tauri && cargo fmt
	@echo "Rust 代码格式化完成！"

# 格式化所有代码
fmt: fmt-rs
	@echo "所有代码格式化完成！"

# Rust 代码检查（clippy）
lint-rs:
	@echo "运行 Rust clippy 检查..."
	cd src-tauri && cargo clippy -- -D warnings
	@echo "Rust 代码检查完成！"

# 所有代码检查
lint:
	@echo "运行 TypeScript 类型检查..."
	npx vue-tsc --noEmit
	@echo "运行 Rust clippy 检查..."
	cd src-tauri && cargo clippy -- -D warnings
	@echo "所有代码检查完成！"

# 运行 Rust 单元测试
test:
	@echo "运行 Rust 单元测试..."
	cd src-tauri && cargo test
	@echo "测试完成！"

# Rust 编译检查
check:
	@echo "运行 Rust 编译检查..."
	cd src-tauri && cargo check
	@echo "编译检查通过！"

# 显示开发环境信息
env-info:
	@echo "=== 开发环境信息 ==="
	@echo "--- Node.js ---"
	@node --version
	@echo "--- npm ---"
	@npm --version
	@echo "--- Rust ---"
	@rustc --version
	@cargo --version
	@echo "--- Tauri CLI ---"
	@npx tauri --version
	@echo "================"

# ============== CLI 模式 ==============

# 构建 zhmm-cli release
cli:
	@echo "构建 zhmm-cli release 二进制..."
	cd src-tauri && cargo build --release --bin zhmm-cli
	@echo ""
	@echo "✓ 构建完成：src-tauri/target/release/zhmm-cli"
	@echo "  下一步：make cli-install  把它装到 PATH 里"

# 安装到 ~/.local/bin（或自定义 CLI_PREFIX）
cli-install: cli
	@mkdir -p "$(CLI_BIN_DIR)"
	@cp src-tauri/target/release/zhmm-cli "$(CLI_BIN)"
	@echo "✓ 已安装：$(CLI_BIN)"
	@case ":$$PATH:" in *":$(CLI_BIN_DIR):"*) ;; *) echo "⚠ 提醒：$(CLI_BIN_DIR) 不在 PATH 中，请把以下行加入 ~/.zshrc："; echo "  export PATH=\"$(CLI_BIN_DIR):\$$PATH\"";; esac
	@echo ""
	@echo "试一下：zhmm-cli --help"

# 卸载
cli-uninstall:
	@rm -f "$(CLI_BIN)"
	@echo "✓ 已卸载：$(CLI_BIN)"

# 速查卡（不用记子命令，直接 make cli-help）
cli-help:
	@echo "================ zhmm-cli 速查卡 ================"
	@echo "通用：每条命令都需要 -f <密库.zmb> -a <账号>"
	@echo "  推荐先 export ZHMM_FILE=~/Documents/my.zmb ZHMM_ACCOUNT=ws"
	@echo "  之后所有命令都可省略 -f / -a；密码用 -p 或 ZHMM_PASSWORD"
	@echo ""
	@echo "-- 库管理 --"
	@echo "  zhmm-cli init                       创建新密库"
	@echo "  zhmm-cli backup                     在 .backups/ 加密备份"
	@echo "  zhmm-cli rekey                      改主密码（自动保险备份）"
	@echo ""
	@echo "-- 查询（id 或关键字都行，关键字按 user/url/desc 子串模糊匹配） --"
	@echo "  zhmm-cli list                       列出全部条目"
	@echo "  zhmm-cli list -q github             关键字搜索（user/url/desc）"
	@echo "  zhmm-cli list -r 工作 -t bank       按 role / tag 过滤"
	@echo "  zhmm-cli get  github                模糊查看（唯一命中时返回）"
	@echo "  zhmm-cli get  1779491977            也可直接传 id"
	@echo "  zhmm-cli get  github -p | pbcopy    仅密码、塞入剪贴板"
	@echo "  zhmm-cli totp github                取一次性验证码"
	@echo ""
	@echo "-- 写入 --"
	@echo "  zhmm-cli add  -u alice --url https://github.com"
	@echo "  zhmm-cli add  -u bob --pwd ''       --pwd '' 表示随机生成 16 位"
	@echo "  zhmm-cli del  github                删除（同样支持关键字或 id）"
	@echo ""
	@echo "-- 导入导出 --"
	@echo "  zhmm-cli export-xlsx out.xlsx"
	@echo "  zhmm-cli import-xlsx in.xlsx"
	@echo ""
	@echo "忘了某个子命令的细节？跑：zhmm-cli help <子命令>"
	@echo "=================================================="
