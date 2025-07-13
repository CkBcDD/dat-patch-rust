use chrono::{Duration, Local, NaiveDateTime};
use regex::Regex;
use std::fs;
use std::io;
use std::path::Path;

/// Cleans up old backup archives based on the keep_months parameter.
///
/// # Arguments
/// * `destination_path` - The directory where backup archives are stored.
/// * `keep_months` - The number of months to keep backups.
/// * `silent` - Suppress console output.
pub fn cleanup_old_backups(
    destination_path: &Path,
    keep_months: u32,
    silent: bool,
) -> io::Result<()> {
    if keep_months == 0 {
        return Ok(());
    }

    // 计算删除的截止日期
    let deadline = Local::now() - Duration::days(30 * keep_months as i64);
    if !silent {
        println!(
            "\nRemoving backups older than {} months (before {})...",
            keep_months,
            deadline.format("%Y-%m-%d %H:%M:%S")
        );
    }

    // 正则表达式，用于匹配文件名并捕获时间戳
    // 例如: "2024-12_backup_20250101123045.zip"
    let re = Regex::new(r"^\d{4}-\d{2}_backup_(\d{14})\.zip$").unwrap();

    for entry in fs::read_dir(destination_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if let Some(caps) = re.captures(file_name) {
                    if let Some(ts_match) = caps.get(1) {
                        let ts_str = ts_match.as_str();
                        // 尝试将时间戳字符串解析为日期时间对象
                        if let Ok(file_timestamp_naive) =
                            NaiveDateTime::parse_from_str(ts_str, "%Y%m%d%H%M%S")
                        {
                            let file_timestamp =
                                file_timestamp_naive.and_local_timezone(Local).unwrap();

                            // 如果文件的时间戳早于截止日期，则删除
                            if file_timestamp < deadline {
                                match fs::remove_file(&path) {
                                    Ok(_) => {
                                        if !silent {
                                            println!("Removed old backup: {}", file_name)
                                        }
                                    }
                                    Err(e) => {
                                        if !silent {
                                            eprintln!("Failed to remove {}: {}", file_name, e)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
