use std::{env, fs};
use std::path::PathBuf;

use anyhow::anyhow;
use scraper::Selector;
use serde::{Deserialize, Serialize};

use crate::operator_logging;
use crate::utils::*;
use crate::vars::CURRENT_VERSION_PATH;
use crate::versions::VersionOperator;

pub const TAG: &str = "go";
pub const DOWNLOAD_URL: &str = "https://go.dev/dl";

#[derive(Debug, Serialize, Deserialize)]
struct Version {
    pub version: String,
    pub stable: bool,
    pub files: Vec<VersionFile>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VersionFile {
    pub filename: String,
    pub os: String,
    pub arch: String,
    pub version: String,
    pub sha256: String,
    pub size: usize,
    pub kind: String,
}

pub struct Entry;

impl Entry {
    async fn list_versions_remote_latest(&self) -> anyhow::Result<Vec<Version>> {
        let url = format!("{}/?mode=json", DOWNLOAD_URL);
        let resp = reqwest::get(&url).await?;
        if !resp.status().is_success() {
            return Err(anyhow!(
                "request '{}' failed, status: {:?} {}",
                url,
                resp.version(),
                resp.status()
            ));
        }
        Ok(serde_json::from_str(resp.text().await?.as_str())?)
    }

    async fn list_versions_remote_archive(&self) -> anyhow::Result<Vec<Version>> {
        let resp = reqwest::get(DOWNLOAD_URL).await?;
        if !resp.status().is_success() {
            return Err(anyhow!(
                "request '{}' failed, status: {:?} {}",
                DOWNLOAD_URL,
                resp.version(),
                resp.status()
            ));
        }
        let doc = scraper::Html::parse_document(resp.text().await?.as_str());
        let selector = Selector::parse("#archive .expanded div.toggle").unwrap();
        Ok(doc
            .select(&selector)
            .into_iter()
            .filter_map(|elem| {
                let id = elem.attr("id");
                if id.is_none() {
                    return None;
                }
                let id = id.unwrap();
                let selector = Selector::parse(".expanded downloadtable tbody tr").unwrap();
                let files: Vec<VersionFile> = elem
                    .select(&selector)
                    .map(|elem| {
                        let selector = Selector::parse("td").unwrap();
                        let mut elem = elem.select(&selector).into_iter();
                        VersionFile {
                            filename: elem.next().unwrap().text().collect(),
                            kind: elem.next().unwrap().text().collect(),
                            os: elem.next().unwrap().text().collect(),
                            arch: elem.next().unwrap().text().collect(),
                            version: id.to_string(),
                            size: elem
                                .next()
                                .unwrap()
                                .text()
                                .collect::<String>()
                                .trim_end_matches("MB")
                                .parse()
                                .unwrap_or(0)
                                * 1024
                                * 1024,
                            sha256: elem.next().unwrap().text().collect(),
                        }
                    })
                    .collect();
                Some(Version {
                    version: id.to_string(),
                    stable: false,
                    files,
                })
            })
            .collect())
    }

    fn match_version_filename(&self, filename: &str) -> bool {
        // Supported windows, macos, linux
        let os = match env::consts::OS {
            "windows" => "windows",
            "macos" => "darwin",
            "linux" => "linux",
            _ => "unknown",
        };
        let arch = match env::consts::ARCH {
            "x86" => "386",
            "x86_64" => "amd64",
            "arm" => "armv6l",
            "aarch64" => "arm64",
            _ => "unknown",
        };
        filename.contains(format!("{}-{}", os, arch).as_str())
    }
}

impl VersionOperator for Entry {
    fn get_versions_dir(&self) -> anyhow::Result<PathBuf> {
        get_versions_dir(TAG)
    }

    async fn list_versions_remote(&self, all: bool) -> anyhow::Result<()> {
        let mut versions_latest = self.list_versions_remote_latest().await?;
        if all {
            versions_latest.extend(self.list_versions_remote_archive().await?.into_iter());
        }
        versions_latest
            .iter()
            .map(|v| v.version.trim_start_matches(TAG))
            .for_each(|v| println!("{}", v));
        Ok(())
    }

    async fn install_version(&self, version: &str) -> anyhow::Result<()> {
        // 获取所有versions
        let mut versions = self.list_versions_remote_latest().await?;
        versions.extend(self.list_versions_remote_archive().await?.into_iter());

        if let Some(target_version) = versions.iter().find(|v| v.version.contains(version)) {
            if let Some(f) = target_version
                .files
                .iter()
                .find(|f| self.match_version_filename(f.filename.as_str()))
            {
                println!("Installing version {}...", f.version.clone());
                // 下载文件
                let url = format!("{}/{}", DOWNLOAD_URL, f.filename.as_str());
                let filepath = download_file(url.as_str(), f.filename.as_str(), f.sha256.as_str()).await?;
                println!("Download file completed '{}'", filepath.display());
                // 解压路径
                let extract_path = self.get_versions_dir()?.join(f.version.trim_start_matches(TAG));
                // 如果目标文件夹已经存在, 先执行删除操作
                if extract_path.exists() { operator_logging!(fs::remove_dir_all(&extract_path)?, format!("Extract path exists and delete '{}'", extract_path.display())) }
                // 解压文件
                let temp_extract_path = decompress_file(&filepath)?;
                // go压缩包特殊处理
                let from_path = temp_extract_path.join(PathBuf::from(TAG));
                fs::rename(&from_path, &extract_path)?;
                // 删除临时目录
                fs::remove_dir_all(&temp_extract_path)?;
                println!("Extract file to '{}'", extract_path.display());
                return Ok(());
            }
        }
        Err(anyhow!("cannot install version({})", version))
    }

    fn uninstall_version(&self, versions: Vec<String>) -> anyhow::Result<()> {
        let ref versions_dir = self.get_versions_dir()?;
        let ref current_version = versions_dir.join(PathBuf::from(CURRENT_VERSION_PATH));
        versions
            .iter()
            .map(|p| versions_dir.join(PathBuf::from(p)))
            .for_each(|ref p| {
                if let Err(e) = fs::remove_dir_all(p) {
                    eprintln!("{}: {}", p.display(), e);
                }
                let _ = check_and_remove_link(current_version, p);
            });
        Ok(())
    }
}
