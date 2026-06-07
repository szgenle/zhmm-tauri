//! 标签注册表模型：TagDefinition
//!
//! 用于定义标签池（一级标签白名单 + 颜色/图标元数据），
//! 存储在 VaultData.tag_registry 中，旧库缺失该字段时反序列化为空 Vec。

use serde::{Deserialize, Serialize};

/// 标签定义条目（注册表中的一条记录）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagDefinition {
    /// 标签名（即实际存储在 entry.tags 中的字符串，唯一键）
    pub name: String,
    /// 显示颜色（CSS hex，如 "#18a058"），空串=默认
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub color: String,
    /// 图标（emoji 或空）
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon: String,
    /// 排序权重（越小越靠前）
    #[serde(default)]
    pub order: i32,
    /// 来源标记："manual"=用户手动创建, "import"=批量导入词典, 空串=未知/旧数据
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
}

impl TagDefinition {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            color: String::new(),
            icon: String::new(),
            order: 0,
            source: String::new(),
        }
    }
}

/// 标签统计信息（返回给前端）
#[derive(Debug, Clone, Serialize)]
pub struct TagStats {
    pub name: String,
    /// 总使用次数（在所有条目的 tags 数组中出现的次数）
    pub count: usize,
    /// 作为一级标签（tags[0]）出现的次数
    pub primary_count: usize,
    /// 是否存在于站点词典中
    pub in_catalog: bool,
}

/// 标签层级节点：一级标签 → 子标签列表（逻辑层级，基于 tags[0] 聚合）
#[derive(Debug, Clone, Serialize)]
pub struct TagHierarchyNode {
    /// 一级标签名（tags[0]）
    pub name: String,
    /// 在该一级标签下出现的子标签（tags[1..] 中去重后的列表）
    pub children: Vec<String>,
}
