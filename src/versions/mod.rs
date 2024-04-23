use std::fs;
use std::path::PathBuf;

use anyhow::anyhow;

use crate::vars::*;

pub mod go;

pub trait VersionOperator {
    fn get_versions_dir(&self) -> anyhow::Result<PathBuf>;

    async fn list_versions_remote(&self, all: bool) -> anyhow::Result<()>;

    async fn install_version(&self, version: &str) -> anyhow::Result<()>;

    fn uninstall_version(&self, versions: Vec<String>) -> anyhow::Result<()>;

    fn list_versions_local(&self) -> anyhow::Result<()> {
        let current_version = match self.get_versions_dir()?.join(CURRENT_VERSION_PATH).read_link() {
            Ok(link) => Some(link),
            Err(_) => None,
        };
        self.get_versions_dir()?.read_dir()?.for_each(|entry| {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() && !path.is_symlink() {
                    let prefix = if let Some(current_version) = &current_version {
                        if current_version.eq(&path) {
                            "* "
                        } else {
                            "  "
                        }
                    } else {
                        "  "
                    };
                    println!("{}{}", prefix, entry.file_name().to_string_lossy());
                }
            }
        });
        Ok(())
    }

    fn use_version(&self, version: &str) -> anyhow::Result<()> {
        let versions_dir = self.get_versions_dir()?;
        // 判断目标version文件夹是否存在
        let target_version_path = versions_dir.clone().join(PathBuf::from(version));
        if !target_version_path.exists() {
            return Err(anyhow!("target version ({}) not found", version));
        }
        // 获取当前version文件夹
        let current_version_path = versions_dir.join(PathBuf::from(CURRENT_VERSION_PATH));
        // 判断当前version文件夹是否存在, 如果存在则需要先删除
        if current_version_path.exists() {
            fs::remove_dir(current_version_path.clone())?;
        }

        // windows系统创建符号链接
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(target_version_path, current_version_path)?;
        // unix系统创建软链接
        #[cfg(unix)]
        std::os::unix::fs::symlink(target_path, current_version_path)?;

        Ok(())
    }
}
