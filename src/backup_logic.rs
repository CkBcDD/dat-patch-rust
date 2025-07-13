use chrono::{Datelike, Local, NaiveDate};

/// 定义备份模式
#[derive(Debug, PartialEq, Eq)]
pub enum BackupMode {
    PreviousMonth,
    CurrentMonth,
    Dynamic,
}

/// 定义要备份的年月
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BackupMonth {
    pub year: i32,
    pub month: u32,
}

/// 根据模式确定需要备份的月份列表
///
/// # Arguments
///
/// * `mode` - 备份模式 (`PreviousMonth`, `CurrentMonth`, `Dynamic`)
///
/// # Returns
///
/// 一个包含 `BackupMonth` 的向量，按时间顺序排列。
pub fn determine_backup_months(mode: &BackupMode) -> Vec<BackupMonth> {
    let today = Local::now().date_naive();
    let mut result = Vec::new();

    let current_month = BackupMonth {
        year: today.year(),
        month: today.month(),
    };

    // 获取上个月的日期
    let first_day_of_current_month =
        NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
    let last_day_of_previous_month = first_day_of_current_month.pred_opt().unwrap();
    let previous_month = BackupMonth {
        year: last_day_of_previous_month.year(),
        month: last_day_of_previous_month.month(),
    };

    match mode {
        BackupMode::PreviousMonth => {
            result.push(previous_month);
        }
        BackupMode::CurrentMonth => {
            result.push(current_month);
        }
        BackupMode::Dynamic => {
            // PowerShell: (New-TimeSpan -Start $lastDayOfPrev -End $today).Days
            // 如果今天与上个月最后一天相差7天以内
            let days_diff = today
                .signed_duration_since(last_day_of_previous_month)
                .num_days();
            if days_diff >= 0 && days_diff <= 7 {
                // 同时备份上个月和当月
                result.push(previous_month);
                result.push(current_month);
            } else {
                // 只备份当月
                result.push(current_month);
            }
        }
    }

    // 确保结果是排序的
    result.sort();
    result
}
