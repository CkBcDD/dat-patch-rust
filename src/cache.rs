use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

/// 定义单个备份事件的记录结构
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct CacheRecord {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub backup_info: String,
}

/// 读取并解析缓存文件
///
/// # Arguments
/// * `cache_path` - `backupEvents.json` 文件的路径
///
/// # Returns
/// 成功时返回一个包含 `CacheRecord` 的向量，如果文件不存在、为空或解析失败，则返回错误。
pub fn read_cache_records(cache_path: &Path) -> io::Result<Vec<CacheRecord>> {
    // 检查文件是否存在
    if !cache_path.exists() {
        return Ok(Vec::new()); // 文件不存在，返回空记录
    }

    let content = fs::read_to_string(cache_path)?;
    if content.trim().is_empty() {
        return Ok(Vec::new()); // 文件为空，返回空记录
    }

    // 解析 JSON
    serde_json::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// 从缓存记录中获取最后一次备份的结束时间
///
/// # Arguments
/// * `records` - `CacheRecord` 的切片
///
/// # Returns
/// 返回最后一次备份的 `EndTime`。如果没有记录，则返回一个10年前的时间点。
pub fn get_last_backup_time(records: &[CacheRecord]) -> DateTime<Utc> {
    records.iter().max_by_key(|r| r.end_time).map_or_else(
        || Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap(), // 如果没有记录，返回一个很早的时间
        |r| r.end_time,
    )
}

/// 将缓存记录列表写入到指定的 JSON 文件。
///
/// # Arguments
/// * `cache_path` - `backupEvents.json` 文件的路径。
/// * `records` - 需要写入的完整记录切片。
pub fn write_cache_records(cache_path: &Path, records: &[CacheRecord]) -> io::Result<()> {
    // 使用 to_string_pretty 来生成格式化、易读的 JSON 文件
    let json_content = serde_json::to_string_pretty(records)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(cache_path, json_content)
}
