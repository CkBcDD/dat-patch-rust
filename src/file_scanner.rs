use crate::backup_logic::BackupMonth;
use chrono::{DateTime, Datelike, Local, TimeZone, Utc};
use std::io;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// 获取指定年月的起止时间（UTC）
fn get_month_range_utc(month: &BackupMonth) -> (DateTime<Utc>, DateTime<Utc>) {
    let start_naive = chrono::NaiveDate::from_ymd_opt(month.year, month.month, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    let end_naive = if month.month == 12 {
        chrono::NaiveDate::from_ymd_opt(month.year + 1, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    } else {
        chrono::NaiveDate::from_ymd_opt(month.year, month.month + 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    };

    // 将本地时间的起止转换为 UTC
    let start_utc = Local
        .from_local_datetime(&start_naive)
        .unwrap()
        .with_timezone(&Utc);
    let end_utc = Local
        .from_local_datetime(&end_naive)
        .unwrap()
        .with_timezone(&Utc);

    (start_utc, end_utc)
}

/// 查找需要备份的文件
///
/// 遍历源目录，找到所有在 `last_backup_time` 之后修改过，
/// 且修改时间在指定月份范围内的文件。
pub fn find_files_to_backup(
    source_path: &Path,
    last_backup_time: &DateTime<Utc>,
    month_to_scan: &BackupMonth,
) -> io::Result<Vec<PathBuf>> {
    let mut files_to_backup = Vec::new();
    let (month_start, month_end) = get_month_range_utc(month_to_scan);

    for entry in WalkDir::new(source_path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let metadata = entry.metadata()?;
            let modified_time: DateTime<Utc> = metadata.modified()?.into();

            if modified_time > *last_backup_time
                && modified_time >= month_start
                && modified_time < month_end
            {
                files_to_backup.push(entry.into_path());
            }
        }
    }

    Ok(files_to_backup)
}
