use std::{env, fs, io};
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;

use anyhow::anyhow;
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use tar::Archive;
use uuid::Uuid;
use zip::ZipArchive;

use crate::vars::*;

pub fn create_dir_if_not_exists(path: &PathBuf) -> anyhow::Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

pub fn get_versions_dir(versions_path: &str) -> anyhow::Result<PathBuf> {
    let path = get_evm_home_dir()?.join(PathBuf::from(versions_path));
    create_dir_if_not_exists(&path)?;
    Ok(path)
}

pub fn get_evm_home_dir() -> anyhow::Result<PathBuf> {
    let path = get_user_home_dir()?.join(EVM_HOME_PATH);
    create_dir_if_not_exists(&path)?;
    Ok(path)
}

pub fn get_user_home_dir() -> anyhow::Result<PathBuf> {
    Ok(match env::var_os("HOME") {
        Some(home) => PathBuf::from(home),
        None => match env::var_os("USERPROFILE") {
            Some(userprofile) => PathBuf::from(userprofile),
            None => return Err(anyhow!("cannot find user home dir")),
        },
    })
}

pub fn check_and_remove_link(current_version: &PathBuf, target: &PathBuf) -> anyhow::Result<()> {
    let real_link = fs::read_link(current_version)?;
    if real_link.eq(target) {
        fs::remove_dir(current_version)?;
    }
    Ok(())
}

pub fn get_evm_download_dir() -> anyhow::Result<PathBuf> {
    let path = get_evm_home_dir()?.join(EVM_DOWNLOAD_PATH);
    create_dir_if_not_exists(&path)?;
    Ok(path)
}

pub async fn download_file(url: &str, filename: &str, checksum: &str) -> anyhow::Result<PathBuf> {
    let filepath = get_evm_download_dir()?.join(PathBuf::from(filename));
    if filepath.exists() {
        let mut file = fs::File::open(&filepath)?;
        let mut file_content = Vec::new();
        file.read_to_end(&mut file_content)?;
        let mut hasher = Sha256::new();
        hasher.update(&file_content);
        let n_checksum = hasher.finalize();
        if hex::encode(n_checksum).eq(checksum) {
            return Ok(filepath);
        }
    }
    let resp = reqwest::get(url).await?;
    if !resp.status().is_success() {
        return Err(anyhow!("request '{}' failed, status: {:?} {}",url,resp.version(),resp.status()));
    }
    let content = resp.bytes().await?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let file_checksum = hasher.finalize();
    let file_checksum = hex::encode(file_checksum);
    if file_checksum.ne(checksum) {
        return Err(anyhow!("file checksum validate failed when downloading..."));
    }
    let mut file = fs::File::options()
        .read(true)
        .write(true)
        .truncate(true)
        .create(true)
        .open(&filepath)?;
    file.write_all(&content)?;
    Ok(filepath)
}

pub fn decompress_file(filepath: &PathBuf) -> anyhow::Result<PathBuf> {
    let file = fs::File::open(filepath)?;
    let ext = filepath.extension().ok_or(anyhow!("Cannot get extension from file: {}", filepath.display()))?;
    // 创建临时目录
    let temp_dir = env::temp_dir().join(PathBuf::from(Uuid::new_v4().to_string()));
    create_dir_if_not_exists(&temp_dir)?;

    // zip文件
    if ext == "zip" {
        let mut archive = ZipArchive::new(BufReader::new(file))?;
        for i in 0..archive.len() {
            let mut temp_file = archive.by_index(i)?;
            let outpath = temp_dir.join(PathBuf::from(temp_file.name()));
            // 如果是文件夹, 并且文件夹不存在, 则创建文件夹
            if temp_file.name().ends_with('/') && !outpath.exists() {
                fs::create_dir_all(&outpath)?;
            } else {
                // 检查文件的父路径是否存在
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p)?;
                    }
                }
                // 如果是文件, 则解压文件
                let mut outfile = fs::File::create(outpath)?;
                io::copy(&mut temp_file, &mut outfile)?;
            }
        }
    } else {
        let mut archive_file = Archive::new(GzDecoder::new(file));
        archive_file.unpack(&temp_dir)?;
    }
    Ok(temp_dir)
}

#[macro_export]
macro_rules! operator_logging {
    ($stmt:expr, $msg:expr) => {
        {
            $stmt;
            println!("{}", $msg);
        }
    };
}
