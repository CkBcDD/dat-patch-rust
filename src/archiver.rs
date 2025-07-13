use crate::backup_logic::BackupMonth;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use zip::write::{FileOptions, ZipWriter};

/// 将文件列表归档到一个 ZIP 文件中
///
/// # Arguments
/// * `base_source_path` - 源文件的根目录 (e.g., --from)
/// * `files_to_backup` - 需要备份的文件绝对路径列表
/// * `destination_path` - 备份文件存放的目标目录 (e.g., --to)
/// * `month` - 当前正在备份的月份，用于命名
///
/// # Returns
/// 成功时返回创建的 ZIP 文件的路径
pub fn create_archive(
    base_source_path: &Path,
    files_to_backup: &[PathBuf],
    destination_path: &Path,
    month: &BackupMonth,
) -> io::Result<PathBuf> {
    // 1. 创建一个唯一的临时目录
    let temp_dir_name = Uuid::new_v4().to_string();
    let temp_path = destination_path.join(&temp_dir_name);
    fs::create_dir_all(&temp_path)?;

    // 2. 复制文件到临时目录，保持目录结构
    for file_path in files_to_backup {
        let relative_path = file_path.strip_prefix(base_source_path).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "File path not in source base")
        })?;
        let dest_file_path = temp_path.join(relative_path);
        if let Some(parent) = dest_file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(file_path, &dest_file_path)?;
    }

    // 3. 创建 ZIP 归档
    let time_stamp = chrono::Local::now().format("%Y%m%d%H%M%S");
    let zip_file_name = format!(
        "{:04}-{:02}_backup_{}.zip",
        month.year, month.month, time_stamp
    );
    let zip_path = destination_path.join(zip_file_name);
    let zip_file = File::create(&zip_path)?;
    let mut zip = ZipWriter::new(zip_file);
    let options: FileOptions<()> =
        FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for entry in walkdir::WalkDir::new(&temp_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        let name = path.strip_prefix(&temp_path).unwrap();
        if path.is_file() {
            zip.start_file(name.to_string_lossy(), options)?;
            let mut f = File::open(path)?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        } else if !name.as_os_str().is_empty() {
            zip.add_directory(name.to_string_lossy(), options)?;
        }
    }
    zip.finish()?;

    // 4. 删除临时目录
    fs::remove_dir_all(&temp_path)?;

    Ok(zip_path)
}
