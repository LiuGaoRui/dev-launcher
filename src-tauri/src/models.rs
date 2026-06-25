//! 数据模型：与数据库表结构对齐的 serde 结构体。
//!
//! 约定（见 docs/02-数据库设计.md、docs/03-命令清单.md）：
//! - `expected_ports` 存为 TEXT（JSON 数组），用 sqlx `Json` 透明映射
//! - `enabled` 存为 INTEGER 0/1，用 bool 映射
//! - 时间字段（create_time / update_time / last_*）存为 TEXT（datetime('now')），用 String 接收

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use strum::Display;

// ===== group =====

/// 项目分组
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Group {
    pub id: i64,
    pub name: String,
    /// 排序值（升序），DDL 中 `order` 是关键字，sqlx 用反引号列别名映射
    #[sqlx(rename = "order")]
    pub order: i32,
    pub create_time: String,
}

/// 新建分组的输入
#[derive(Debug, Clone, Deserialize)]
pub struct GroupInput {
    pub name: String,
}

/// 更新分组的输入：name / order 均可选
#[derive(Debug, Clone, Deserialize, Default)]
pub struct GroupUpdate {
    pub name: Option<String>,
    pub order: Option<i32>,
}

// ===== project =====

/// 项目类型枚举，与 DDL CHECK 约束一一对应。
///
/// `serde` / `sqlx` / `strum` 三方统一 `serialize_all = "snake_case"`，
/// 字符串表示与 DB 存储值、前端 TS 字面量一致。
/// `Display` 派生自动提供 `to_string()`，替代手写 `as_str()`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Display)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ProjectType {
    Springboot,
    JavaJar,
    Node,
    DockerCompose,
    Custom,
}

impl ProjectType {
    pub fn as_str(self) -> &'static str {
        // strum::Display 自动生成 to_string() 返回 snake_case，
        // 但 as_str() 仍需返回 &'static str 用于零分配场景。
        // 此处保留 match 作为编译期验证：所有变体被覆盖，新增变体时编译报错。
        match self {
            ProjectType::Springboot => "springboot",
            ProjectType::JavaJar => "java_jar",
            ProjectType::Node => "node",
            ProjectType::DockerCompose => "docker_compose",
            ProjectType::Custom => "custom",
        }
    }
}

impl Default for ProjectType {
    fn default() -> Self {
        ProjectType::Custom
    }
}

/// 被管理的项目
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub group_id: Option<i64>,
    pub r#type: ProjectType,
    pub path: String,
    pub start_cmd: String,
    pub build_cmd: Option<String>,
    /// 端口列表（JSON 数组存 TEXT 列）
    #[sqlx(json)]
    pub expected_ports: Vec<String>,
    pub enabled: bool,
    pub last_pid: Option<i64>,
    pub last_start_time: Option<String>,
    pub last_stop_time: Option<String>,
    pub create_time: String,
    pub update_time: String,
}

/// 新建/更新项目的输入（不含 id / 时间戳，由 DB 生成）
#[derive(Debug, Clone, Deserialize)]
pub struct ProjectInput {
    pub name: String,
    pub group_id: Option<i64>,
    pub r#type: ProjectType,
    pub path: String,
    pub start_cmd: String,
    pub build_cmd: Option<String>,
    #[serde(default)]
    pub expected_ports: Vec<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// 当前 ISO8601 时间戳（用于 update_time 手动刷新等场景）
#[allow(dead_code)] // 阶段 2 起进程托管使用
pub fn now_iso() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}
