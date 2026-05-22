# 安全策略 / Security Policy

`zhmm-tauri` 是一款处理用户密码数据的工具，我们非常重视任何安全问题。

---

## 📣 报告安全漏洞 / Reporting a Vulnerability

**请不要在公开 Issue 中披露安全漏洞。**
Please do **not** disclose security vulnerabilities through public GitHub issues.

推荐通过以下任一渠道进行**私下**披露：

1. **GitHub Security Advisory（推荐）**：
   在本仓库 `Security` → `Report a vulnerability` 发起 private advisory
   → <https://github.com/szgenle/zhmm-tauri/security/advisories/new>
2. 在 GitHub 仓库主页通过维护者 profile 中公开的联系方式私下沟通

请在报告中包含：

- 漏洞的描述、影响范围、严重程度评估（若可）
- 复现步骤（建议附带最小化 PoC）
- 受影响的版本（应用版本 / 提交 SHA）
- 你的联系方式（可选，用于后续沟通与致谢）

### 响应时间 / Response Timeline

- **72 小时内** 确认收到报告
- **7 天内** 给出初步分析结论
- **30 天内** 发布修复版本（视严重程度可提前或延后）

修复后，我们会在 Release Notes 的 `Security` 节记录（默认致谢报告者，除非你希望匿名）。

---

## ✅ 支持的版本 / Supported Versions

| 版本     | 是否接受安全更新 |
|---------|-------------------|
| 最新发行版 | ✅                |
| 其它版本   | ❌（请升级）       |

项目尚处于快速迭代期，仅承诺对最新发行版与 `main` 分支提供安全修复。

---

## 🧭 威胁模型说明 / Threat Model

`zhmm-tauri` 的设计目标是：

- 保护静态存储的密码数据（`.zmb` 文件）
- 防止攻击者在**没有主密码 + 账号名**的情况下读取密码内容
- 通过 SM4-GCM 认证标签（v6）/ HMAC-SM3（v5 兼容读）检测文件篡改
- 与 Python 版 `zhmm` 共享同一份 `.zmb` 二进制格式，双方可互相打开同一文件

**不在威胁模型内**的场景（用户需自行负责）：

- 运行时内存被 dump（例如你的机器已被 root）
- 剪贴板被其它应用读取
- 键盘被键盘记录器监听
- 主密码被肩窥、暴力破解（弱密码）
- `.zmb` 文件被反复拿到后离线暴力破解（请使用强主密码 + 非通用账号名）
- WebView2 / WKWebView 等系统 WebView 自身的 0day（属操作系统厂商责任）

---

## 🔐 加密算法详解 / Encryption Details

### v6 算法栈（默认写）

| 环节 | 算法 | 参数 |
|------|------|------|
| 密钥派生 | **Argon2id**（memory-hard） | 默认 `m=64 MiB, t=3, p=1`，16 字节随机盐，输出 32 字节，前 16 字节作 SM4 密钥；KDF 口令材料为 `account.utf8 ‖ 0x00 ‖ password.utf8`；参数随密文头部内嵌存储，允许未来调强度而不破坏老文件 |
| 加密 + 认证 | **SM4-GCM**（NIST SP 800-38D） | 12 字节随机 IV（96-bit），CTR 流加密 + GHASH 认证，**整个 header（含 Argon2 参数与 iv/salt）作为 AAD**，16 字节认证标签 |

> Rust 生态没有现成的 SM4-GCM crate，本项目基于 RustCrypto `sm4` 单块原语自实现 GCM（CTR + GHASH），仅支持 96-bit IV 与 128-bit tag，与 Python 版 `gmssl` 二进制完全一致。实现位于 [src-tauri/src/crypto.rs](src-tauri/src/crypto.rs)。

### v5 算法栈（仅读，自动升级）

为兼容历史 `.zmb` 文件，仍保留 v5 解密能力：

| 环节 | 算法 | 参数 |
|------|------|------|
| 密钥派生 | **Argon2id** | 同 v6（参数从文件头读取） |
| 数据加密 | **SM4-CBC** | 16 字节随机 IV，PKCS7 填充，前 16 字节派生密钥 |
| 完整性校验 | **HMAC-SM3** | 覆盖 `magic + ver + m_cost + t_cost + p_cost + salt + iv + ciphertext`，32 字节标签，第 17~32 字节派生密钥 |

读取 v5 文件后，**下一次保存自动以 v6 格式重新落盘**，逐步淘汰旧格式。

### 文件格式（v6）

```
magic(4B="ZHMM") | ver(1B=6) | m_cost(4B BE) | t_cost(4B BE) | p_cost(4B BE)
                 | salt(16B) | iv(12B) | ciphertext(NB) | tag(16B)
```

- **magic**：固定 4 字节 `ZHMM`，用于文件类型识别
- **ver**：单字节版本号（当前 = 6），便于未来升级
- **m_cost / t_cost / p_cost**：大端无符号 32 位整数，从文件读取的参数优先于默认值，且读取前校验在安全范围内（`m ∈ [8, 524288]` KiB，`t ∈ [1, 100]`，`p ∈ [1, 64]`）以防恶意 blob OOM
- **salt**：每次保存重新生成，确保相同账号+密码产生不同密钥
- **iv**：每次保存重新生成（96-bit GCM IV），确保相同明文产生不同密文
- **tag**：GCM 认证标签覆盖 header（含 Argon2 参数）+ ciphertext，篡改任何字段均会被检测
- **账号名**：作为 KDF 输入的一部分参与密钥派生，**本身不写入文件**；解密时需由调用方重新提供，账号错误将与密码错误产生相同的 GCM 认证失败

### 设计理由

1. **为什么让账号参与 KDF**：账号作为应用层常量盐，使不同账号 + 相同弱密码的用户派生出完全不同的密钥，缓解弱口令用户面临的离线字典/彩虹表风险。
2. **为什么选 Argon2id**：Argon2id 是 2015 年 Password Hashing Competition 冠军算法，memory-hard 特性使其在 GPU/ASIC 上的并行加速比 PBKDF2 困难得多；OWASP 2024 Password Storage Cheat Sheet 明确推荐 Argon2id 作为首选 KDF。
3. **为什么头部内嵌 Argon2 参数**：让默认强度未来可调（硬件提升、安全形势演化）无需再次 bump 文件格式版本；老文件仍能用自己原始参数被正确解密。
4. **为什么从 v5 (CBC+HMAC) 升级到 v6 (GCM)**：GCM 是公认的 AEAD 模式，单一原语同时承担机密性与完整性，header 通过 AAD 同步认证，避免 Encrypt-then-MAC 的边界错误风险。
5. **为什么仍坚持国密 SM3/SM4**：SM3 / SM4 是中国国家标准（GB/T 32905、32907），适合需要国密合规的场景；与 Python 版保持算法栈一致，便于互通。

---

## 🔑 TOTP 2FA 实现说明 / TOTP Implementation

`zhmm-tauri` 内置 TOTP（基于时间的一次性密码）动态码能力，用于承担账号「第二因子」：

| 项目 | 说明 |
|------|------|
| 标准算法 | 完整实现 **RFC 6238**（TOTP）+ **RFC 4226**（HOTP 动态截断），支持 `HMAC-SHA1 / SHA256 / SHA512` |
| 国密扩展 | 新增 **HMAC-SM3** 变体（算法名 `SM3`），与 Python 版完全一致，兼容国密合规场景 |
| Secret 来源 | 支持 Base32 手动粘贴（容错空格、大小写、缺失 padding）与 `otpauth://` URI 解析（自动回填 algo / digits / period） |
| 默认参数 | `digits=6, period=30`，与主流认证器（Google Authenticator / Microsoft Authenticator / 1Password）完全互通；`SM3` 为本项目与 Python 版的私有扩展，其它应用不识别 |
| 刷新节奏 | 表格列每秒重算一次，展示 `当前码 + 剩余秒数`；点击即复制到剪贴板 |

### TOTP Secret 的存储策略

- **`.zmb` 密库**：TOTP Secret 作为条目字段之一，随整库一起经 **Argon2id → SM4-GCM** 链路加密落盘。**破解 TOTP Secret 的门槛与破解主密码完全等价。**
- **Excel 导出（`.xlsx`）**：**刻意不包含 TOTP Secret 列**。导出文件仅保留 `totp_algo / totp_digits / totp_period` 三列元信息，便于迁移时提示「此条目曾启用 2FA，请重新扫码绑定」。这一设计避免用户把明文 Secret 泄露给云盘 / 协作工具 / 邮箱等不受控通道。
- **历史密码**：每条目最多保留最近 5 次旧密码，仅随 `.zmb` 加密落盘，Excel 通道刻意不承载。

### 已知边界

1. **TOTP 并非替代主密码**：与主密码同库存储，本质仍是「你拥有的 `.zmb` 文件 + 你记住的主密码」的加强，**不构成独立第二因子**。若追求「物理隔离第二因子」请使用硬件令牌（YubiKey）或手机认证器。
2. **SM3-TOTP 是私有扩展**：其它认证器不识别 `algorithm=SM3`；请勿将此类 Secret 同时录入到第三方应用。
3. **时间漂移**：本地系统时间偏差 > 30 秒会导致动态码失效，TOTP 校验方通常允许 ±1 个 period 的容差。

---

## ⚠️ 已知局限 / Known Limitations

我们开诚布公地列出已知安全局限，欢迎贡献改进：

1. **JS 主密码字符串**：前端在登录对话框中以普通字符串持有主密码并通过 Tauri IPC 传递给后端；JS 引擎不保证字符串对象被及时清零。后端 Rust 收到后会以 `Vec<u8>` 缓存并在 `Drop` / `lock` 时 `zeroize` 清空。
2. **WebView 上下文**：前端运行在系统 WebView（macOS WKWebView / Windows WebView2 / Linux WebKitGTK）中，受限于该 WebView 的安全实现；本项目通过 Tauri 默认 capability 严格限制 IPC 命令集，不开放任意 fs / shell 能力。
3. **防截屏覆盖范围**：通过 [`anti_capture.rs`](src-tauri/src/anti_capture.rs) 在 macOS / Windows 10 2004+ 平台调用系统 API 让窗口对系统截图/录屏黑屏，**无法防御摄像头翻拍、外接采集卡、虚拟机抓屏、内核级屏幕驱动 hook**。Linux 无可靠系统 API，为 no-op。
4. **自动锁定的粒度有限**：基于窗口活跃状态判断，不监听鼠标/键盘输入；窗口保持前台但长时间无人操作的场景不会触发锁定。
5. **无多因素认证支持**（主密码之外）。
6. **构建产物未签名**：当前 Release 提供的二进制未做代码签名 / 公证，macOS 用户首次启动可能触发 Gatekeeper 警告。请通过 `shasum -a 256` 校验下载产物的 SHA256，再执行：
   ```bash
   xattr -dr com.apple.quarantine /Applications/zhmm-tauri.app
   ```
   建议高安全敏感用户从源码自行构建。

---

## 🛡 使用建议 / Best Practices for Users

- 主密码至少 **16 位**，混合大小写字母、数字、符号
- 账号名避免使用通用字符串（如 `admin`、`root`、`test`），推荐用邮箱、手机号或自定义唯一串，以最大化 KDF 常量盐的熵
- `.zmb` 文件**多地备份**（至少 1 份本地、1 份离线介质）
- 切勿通过聊天软件、邮件传输 `.zmb` 文件或主密码
- 不要在他人设备上运行未经审计的 zhmm-tauri 构建
- 下载 Release 产物后请验证 SHA256 校验和

---

感谢你帮助让 `zhmm-tauri` 变得更安全。🙏
