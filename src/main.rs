use chrono::Utc;
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::process;

mod archiver;
mod backup_logic;
mod cache;
mod cleaner;
mod file_scanner;

use backup_logic::{BackupMode, determine_backup_months};

/// Incremental backup script for WeChat data, rewritten in Rust.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The source path (WeChat root directory) to back up.
    #[arg(long)]
    from: PathBuf,

    /// The destination path for storing backup .zip and .cache.
    #[arg(long)]
    to: PathBuf,

    /// Backup the previous month.
    #[arg(short, long, group = "mode")]
    p: bool,

    /// Backup the current month.
    #[arg(short, long, group = "mode")]
    n: bool,

    /// Dynamic mode: backup based on date proximity to the end of the month.
    #[arg(short, long, group = "mode")]
    d: bool,

    /// Silent mode: suppress console output.
    #[arg(short, long)]
    s: bool,

    /// The number of months to keep backups.
    #[arg(long, default_value_t = 6)]
    keep_months: u32,
}

fn main() {
    let args = Args::parse();
    let script_start_time = Utc::now(); // 1. 记录脚本开始时间

    // 0. 预检查
    if !args.from.exists() {
        // 关键错误信息即使在静默模式下也应该显示
        eprintln!(
            "Error: The source path '{}' does not exist.",
            args.from.display()
        );
        process::exit(1);
    }
    if !args.to.exists() {
        if !args.s {
            println!(
                "Warning: The destination path '{}' does not exist. Creating...",
                args.to.display()
            );
        }
        if let Err(e) = fs::create_dir_all(&args.to) {
            eprintln!("Error: Failed to create destination directory: {}", e);
            process::exit(1);
        }
    }

    // 1. 根据参数确定备份模式
    let mode = if args.p {
        BackupMode::PreviousMonth
    } else if args.n {
        BackupMode::CurrentMonth
    } else if args.d {
        BackupMode::Dynamic
    } else {
        // Clap 的 group 设置应该能防止这种情况，但作为安全措施我们还是处理一下
        eprintln!("Error: You must specify exactly one of -p, -n, or -d.");
        process::exit(1);
    };

    // 2. 计算需要备份的月份
    let months_to_backup = determine_backup_months(&mode);

    if months_to_backup.is_empty() {
        if !args.s {
            println!("No months to backup based on the selected mode. Exiting.");
        }
        return;
    }

    // 3. 读取 .cache 并获取上次备份时间
    let cache_folder = args.to.join(".cache");
    if !cache_folder.exists() {
        if let Err(e) = fs::create_dir_all(&cache_folder) {
            eprintln!("Error: Failed to create .cache directory: {}", e);
            process::exit(1);
        }
    }
    let cache_file = cache_folder.join("backupEvents.json");

    let mut cache_records = match cache::read_cache_records(&cache_file) {
        // 声明为可变
        Ok(records) => records,
        Err(e) => {
            eprintln!("Error reading cache file '{}': {}", cache_file.display(), e);
            process::exit(1);
        }
    };

    let last_backup_time = cache::get_last_backup_time(&cache_records);

    if !args.s {
        println!("Arguments parsed successfully:");
        println!("{:#?}", args);
        println!("\nSelected backup mode: {:?}", mode);
        println!("Months to be backed up: {:?}", months_to_backup);
        println!(
            "Last backup time from cache: {}",
            last_backup_time.with_timezone(&chrono::Local)
        );
        println!("\nStarting file scan...");
    }

    // 用于跟踪本次运行是否真的创建了备份
    let mut archives_created_this_run = false;

    // 4. 遍历每个待备份月份，查找文件并归档
    for month in &months_to_backup {
        if !args.s {
            println!(
                "Scanning for new/updated files for month: {:04}-{:02}...",
                month.year, month.month
            );
        }

        match file_scanner::find_files_to_backup(&args.from, &last_backup_time, month) {
            Ok(files) => {
                if files.is_empty() {
                    if !args.s {
                        println!(
                            "No new or updated files found for {:04}-{:02}. Skipping.",
                            month.year, month.month
                        );
                    }
                } else {
                    if !args.s {
                        println!(
                            "Found {} files to backup for {:04}-{:02}. Archiving...",
                            files.len(),
                            month.year,
                            month.month
                        );
                    }

                    match archiver::create_archive(&args.from, &files, &args.to, month) {
                        Ok(zip_path) => {
                            if !args.s {
                                println!("Successfully created archive: {}", zip_path.display());
                            }
                            archives_created_this_run = true; // 标记已成功创建归档
                        }
                        Err(e) => {
                            if !args.s {
                                eprintln!(
                                    "Error creating archive for {:04}-{:02}: {}",
                                    month.year, month.month, e
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                if !args.s {
                    eprintln!(
                        "Error scanning files for {:04}-{:02}: {}",
                        month.year, month.month, e
                    );
                }
            }
        }
    }

    // 6. 滚动删除旧备份
    if args.keep_months > 0 {
        if let Err(e) = cleaner::cleanup_old_backups(&args.to, args.keep_months, args.s) {
            if !args.s {
                eprintln!("\nAn error occurred during cleanup: {}", e);
            }
        }
    }

    // 5. 如果创建了新的备份，则更新 .cache 文件
    if !archives_created_this_run {
        if !args.s {
            println!("\nNo new backup archives were created. Cache will not be updated.");
            println!("\nBackup process completed.");
        }
        return; // 现在可以安全退出
    }

    let script_end_time = Utc::now();
    let backup_month_info = months_to_backup
        .iter()
        .map(|m| format!("{:04}-{:02}", m.year, m.month))
        .collect::<Vec<_>>()
        .join(", ");

    let new_record = cache::CacheRecord {
        start_time: script_start_time,
        end_time: script_end_time,
        backup_info: format!("Backup for {}", backup_month_info),
    };

    cache_records.push(new_record);

    match cache::write_cache_records(&cache_file, &cache_records) {
        Ok(_) => {
            if !args.s {
                println!(
                    "\nSuccessfully updated cache file: {}",
                    cache_file.display()
                )
            }
        }
        Err(e) => {
            if !args.s {
                eprintln!("\nError writing to cache file: {}", e)
            }
        }
    }

    if !args.s {
        println!("\nBackup process completed.");
    }
}
