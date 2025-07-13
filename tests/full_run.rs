use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime};

// 辅助函数：设置文件的修改时间
fn set_file_mtime(path: &PathBuf, time: SystemTime) {
    filetime::set_file_mtime(path, filetime::FileTime::from_system_time(time)).unwrap();
}

#[test]
fn test_full_backup_and_cleanup_flow() {
    // --- 1. SETUP ---
    // 创建一个临时的、唯一的测试根目录
    let test_root = std::env::temp_dir().join(format!("dat-patch-test-{}", uuid::Uuid::new_v4()));
    let source_dir = test_root.join("in");
    let dest_dir = test_root.join("out");
    let cache_dir = dest_dir.join(".cache");

    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&cache_dir).unwrap();

    // 模拟上次备份时间为 20 天前
    let last_backup_sys_time = SystemTime::now() - Duration::from_secs(20 * 24 * 3600);
    let last_backup_utc: chrono::DateTime<chrono::Utc> = last_backup_sys_time.into();
    let initial_cache_content = format!(
        r#"[{{ "StartTime": "2025-01-01T00:00:00Z", "EndTime": "{}", "BackupInfo": "Initial" }}]"#,
        last_backup_utc.to_rfc3339()
    );
    fs::write(cache_dir.join("backupEvents.json"), initial_cache_content).unwrap();

    // 创建一个应该被清理的旧备份 (4个月前)
    let old_backup_name = "2025-03_backup_20250310100000.zip";
    fs::write(dest_dir.join(old_backup_name), "old").unwrap();

    // 创建一个应该被保留的新备份 (2个月前)
    let recent_backup_name = "2025-05_backup_20250515100000.zip";
    fs::write(dest_dir.join(recent_backup_name), "recent").unwrap();

    // 创建一个应该被备份的新文件 (10天前)
    let new_file_path = source_dir.join("new_file.txt");
    fs::write(&new_file_path, "new content").unwrap();
    set_file_mtime(&new_file_path, SystemTime::now() - Duration::from_secs(10 * 24 * 3600));

    // 创建一个不应被备份的旧文件 (30天前)
    let old_file_path = source_dir.join("old_file.txt");
    fs::write(&old_file_path, "old content").unwrap();
    set_file_mtime(&old_file_path, SystemTime::now() - Duration::from_secs(30 * 24 * 3600));

    // --- 2. EXECUTION ---
    // 获取 cargo build 的可执行文件路径
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_dat-patch-rust"));
    cmd.arg("--from")
        .arg(&source_dir)
        .arg("--to")
        .arg(&dest_dir)
        .arg("-n") // 备份当月
        .arg("--keep-months")
        .arg("3"); // 保留3个月

    let output = cmd.output().expect("Failed to execute command");
    assert!(output.status.success(), "Command executed with error: {:?}", String::from_utf8_lossy(&output.stderr));

    // --- 3. ASSERTION ---
    // 3.1 验证清理
    let dest_files: Vec<String> = fs::read_dir(&dest_dir)
        .unwrap()
        .map(|res| res.unwrap().file_name().into_string().unwrap())
        .collect();

    assert!(!dest_files.contains(&old_backup_name.to_string()), "Old backup was not deleted");
    assert!(dest_files.contains(&recent_backup_name.to_string()), "Recent backup was deleted");

    // 3.2 验证新备份
    let new_backup_file = dest_files
        .iter()
        .find(|name| name.contains("_backup_") && !name.contains("2025-03") && !name.contains("2025-05"));
    assert!(new_backup_file.is_some(), "No new backup archive was created");

    // 3.3 验证缓存更新
    let final_cache_content = fs::read_to_string(cache_dir.join("backupEvents.json")).unwrap();
    let final_records: Vec<serde_json::Value> = serde_json::from_str(&final_cache_content).unwrap();
    assert_eq!(final_records.len(), 2, "Cache file was not updated with a new record");

    // --- 4. TEARDOWN ---
    fs::remove_dir_all(&test_root).unwrap();
}