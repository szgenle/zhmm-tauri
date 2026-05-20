.PHONY: help install dev dev-fe build build-debug clean clean-fe clean-rs clean-all fmt fmt-rs lint lint-rs test check env-info

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
