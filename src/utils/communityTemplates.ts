/**
 * 社区模板预设包（v2.0+ alpha.3 数据交换）
 *
 * 内置一组常见场景的精选模板包，用户在「模板管理 → 浏览社区模板」中
 * 一键装入即可使用。社区预设的字段定义与 match_rules 都经过设计，
 * 添加后即可享受 alpha.2 的自动推荐能力。
 *
 * 设计取舍：
 *  - 当前 alpha 阶段直接以 TS 常量内置，免去文件分发与远程拉取的复杂度
 *  - 后续可平滑切换到「从 GitHub 仓库 fetch JSON 模板包」的方案，
 *    数据结构与本地导入接受的 TemplatePack 完全一致
 *  - 这里不重复内建 4 个默认模板（bank_card / id_card / work_internal / game），
 *    那些由 default_templates 提供
 */
import type { AccountTemplate } from "../api";

/** 社区模板包元信息：用于 UI 列表展示 */
export interface CommunityTemplatePack {
  /** 包级 id（前端展示用，与模板 id 不同） */
  id: string;
  /** 包名 */
  name: string;
  /** 一句话介绍 */
  description: string;
  /** 单字符 emoji 图标 */
  icon: string;
  /** 包内模板（与导入接口接受的格式一致） */
  templates: AccountTemplate[];
}

export const COMMUNITY_PACKS: CommunityTemplatePack[] = [
  {
    id: "developer_tools",
    name: "开发者工具",
    description: "GitHub / GitLab / Docker Hub / npm / PyPI 等开发者常用平台",
    icon: "💻",
    templates: [
      {
        id: "dev_github",
        name: "GitHub",
        icon: "🐙",
        fields: [
          { key: "username", label: "用户名", field_type: "text", required: true },
          { key: "ssh_key_path", label: "SSH Key 路径", field_type: "text" },
          { key: "personal_token", label: "Personal Access Token", field_type: "secret" },
          { key: "recovery_codes", label: "二步验证恢复码", field_type: "multiline" },
        ],
        match_rules: [
          { kind: "url_contains", value: "github.com" },
          { kind: "keyword", value: "github" },
        ],
      },
      {
        id: "dev_gitlab",
        name: "GitLab",
        icon: "🦊",
        fields: [
          { key: "username", label: "用户名", field_type: "text", required: true },
          { key: "personal_token", label: "Personal Access Token", field_type: "secret" },
          { key: "deploy_token", label: "Deploy Token", field_type: "secret" },
        ],
        match_rules: [
          { kind: "url_contains", value: "gitlab.com" },
          { kind: "keyword", value: "gitlab" },
        ],
      },
      {
        id: "dev_npm",
        name: "npm Registry",
        icon: "📦",
        fields: [
          { key: "username", label: "用户名", field_type: "text", required: true },
          { key: "auth_token", label: "Auth Token", field_type: "secret" },
          { key: "publish_otp", label: "发布 2FA 备份码", field_type: "multiline" },
        ],
        match_rules: [
          { kind: "url_contains", value: "npmjs.com" },
          { kind: "keyword", value: "npm" },
        ],
      },
      {
        id: "dev_dockerhub",
        name: "Docker Hub",
        icon: "🐳",
        fields: [
          { key: "username", label: "用户名", field_type: "text", required: true },
          { key: "access_token", label: "Access Token", field_type: "secret" },
        ],
        match_rules: [
          { kind: "url_contains", value: "hub.docker.com" },
          { kind: "keyword", value: "docker" },
        ],
      },
    ],
  },
  {
    id: "cloud_providers",
    name: "云服务商",
    description: "AWS / 阿里云 / 腾讯云：AccessKey 与控制台登录信息",
    icon: "☁️",
    templates: [
      {
        id: "cloud_aws",
        name: "AWS",
        icon: "🟧",
        fields: [
          { key: "account_id", label: "Account ID", field_type: "text", required: true },
          { key: "iam_user", label: "IAM 用户名", field_type: "text" },
          { key: "access_key_id", label: "Access Key ID", field_type: "text" },
          { key: "secret_access_key", label: "Secret Access Key", field_type: "secret" },
          { key: "mfa_serial", label: "MFA Serial", field_type: "text" },
        ],
        match_rules: [
          { kind: "url_contains", value: "aws.amazon.com" },
          { kind: "url_contains", value: "amazonaws.com" },
          { kind: "keyword", value: "AWS" },
        ],
      },
      {
        id: "cloud_aliyun",
        name: "阿里云",
        icon: "🟦",
        fields: [
          { key: "account", label: "主账号", field_type: "text", required: true },
          { key: "ram_user", label: "RAM 子账号", field_type: "text" },
          { key: "access_key_id", label: "AccessKey ID", field_type: "text" },
          { key: "access_key_secret", label: "AccessKey Secret", field_type: "secret" },
        ],
        match_rules: [
          { kind: "url_contains", value: "aliyun.com" },
          { kind: "url_contains", value: "alibabacloud.com" },
          { kind: "keyword", value: "阿里云" },
        ],
      },
      {
        id: "cloud_tencent",
        name: "腾讯云",
        icon: "🟩",
        fields: [
          { key: "account_id", label: "APPID", field_type: "text", required: true },
          { key: "secret_id", label: "SecretId", field_type: "text" },
          { key: "secret_key", label: "SecretKey", field_type: "secret" },
        ],
        match_rules: [
          { kind: "url_contains", value: "cloud.tencent.com" },
          { kind: "keyword", value: "腾讯云" },
        ],
      },
    ],
  },
  {
    id: "social_media",
    name: "社交媒体",
    description: "微博 / 知乎 / 小红书 / Twitter / 微信公众号 等社交平台",
    icon: "💬",
    templates: [
      {
        id: "social_weibo",
        name: "微博",
        icon: "🅦",
        fields: [
          { key: "uid", label: "UID", field_type: "text" },
          { key: "nickname", label: "昵称", field_type: "text" },
        ],
        match_rules: [
          { kind: "url_contains", value: "weibo.com" },
          { kind: "keyword", value: "微博" },
        ],
      },
      {
        id: "social_zhihu",
        name: "知乎",
        icon: "🐳",
        fields: [
          { key: "nickname", label: "昵称", field_type: "text" },
        ],
        match_rules: [
          { kind: "url_contains", value: "zhihu.com" },
          { kind: "keyword", value: "知乎" },
        ],
      },
      {
        id: "social_xhs",
        name: "小红书",
        icon: "📕",
        fields: [
          { key: "nickname", label: "昵称", field_type: "text" },
          { key: "rednum", label: "小红书号", field_type: "text" },
        ],
        match_rules: [
          { kind: "url_contains", value: "xiaohongshu.com" },
          { kind: "keyword", value: "小红书" },
        ],
      },
      {
        id: "social_wx_mp",
        name: "微信公众平台",
        icon: "📢",
        fields: [
          { key: "appid", label: "AppID", field_type: "text", required: true },
          { key: "appsecret", label: "AppSecret", field_type: "secret" },
          { key: "admin_account", label: "管理员账号", field_type: "text" },
        ],
        match_rules: [
          { kind: "url_contains", value: "mp.weixin.qq.com" },
          { kind: "keyword", value: "公众号" },
          { kind: "keyword", value: "公众平台" },
        ],
      },
    ],
  },
  {
    id: "ecommerce",
    name: "电商平台",
    description: "淘宝 / 京东 / 拼多多 / 亚马逊：店铺主账号或购物账号",
    icon: "🛒",
    templates: [
      {
        id: "ecom_taobao",
        name: "淘宝/天猫",
        icon: "🅣",
        fields: [
          { key: "nickname", label: "会员名", field_type: "text", required: true },
          { key: "shop_name", label: "店铺名（如有）", field_type: "text" },
          { key: "alipay_bind", label: "绑定支付宝", field_type: "text" },
        ],
        match_rules: [
          { kind: "url_contains", value: "taobao.com" },
          { kind: "url_contains", value: "tmall.com" },
          { kind: "keyword", value: "淘宝" },
          { kind: "keyword", value: "天猫" },
        ],
      },
      {
        id: "ecom_jd",
        name: "京东",
        icon: "🅹",
        fields: [
          { key: "nickname", label: "用户名", field_type: "text", required: true },
          { key: "shop_name", label: "店铺名（如有）", field_type: "text" },
        ],
        match_rules: [
          { kind: "url_contains", value: "jd.com" },
          { kind: "keyword", value: "京东" },
        ],
      },
      {
        id: "ecom_amazon",
        name: "Amazon",
        icon: "🅐",
        fields: [
          { key: "country_site", label: "站点（com/cn/jp 等）", field_type: "text" },
          { key: "prime_until", label: "Prime 到期", field_type: "date" },
        ],
        match_rules: [
          { kind: "url_contains", value: "amazon." },
          { kind: "keyword", value: "Amazon" },
          { kind: "keyword", value: "亚马逊" },
        ],
      },
    ],
  },
];
