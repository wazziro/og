use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::NaiveDate;

// A.2.1. 必須キー
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Task {
    pub name: String,
    pub status: String, // TODO: Enum (NONE, PENDING, DOING, WAITING, DONE, CANCELLED, UNKNOWN)
    pub priority: String, // TODO: Enum or specific validation (N, A-Z+)
    pub id: i64, // 1以上の整数
    pub created: NaiveDate, // YYYY-MM-DD
    pub display_order: i64, // 正の整数

    // A.2.2. キーは必須、値は null を許容する項目
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due: Option<NaiveDate>, // YYYY-MM-DD or null
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<NaiveDate>, // YYYY-MM-DD or null
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<NaiveDate>, // YYYY-MM-DD or null

    // A.2.3. オプションキー
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtasks: Option<Vec<Task>>, // 再帰的な構造
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat: Option<RepeatInfo>, // 初期仕様では空オブジェクト {}
}

// repeat フィールド用の構造体 (A.2.3)
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RepeatInfo {
    // 将来的に頻度等のルールを格納
    // 初期仕様ではフィールドなし
}
