use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub struct SyncParams<'a> {
    pub local_dir: &'a str,
    pub remote_dir: &'a str,
    pub host: &'a str,
    pub port: Option<u16>,
    pub ssh_user: &'a str,
    pub identity_file: Option<&'a PathBuf>,
    pub excludes: &'a [String],
    pub delete: bool,
    pub dry_run: bool,
}

pub fn run_rsync(params: &SyncParams) -> Result<(), Box<dyn std::error::Error>> {
    let local_path = Path::new(params.local_dir);

    // 1. フォルダーの存在・ディレクトリ確認
    if !local_path.exists() || !local_path.is_dir() {
        return Err(format!(
            "エラー: 同期元ディレクトリ '{}' が存在しません。",
            params.local_dir
        )
        .into());
    }

    // 2. フォルダー内が空かどうか確認
    let mut entries = fs::read_dir(local_path)?;
    if entries.next().is_none() {
        return Err(format!(
            "エラー: 同期元ディレクトリ '{}' 内にファイルやフォルダーが存在しません。",
            params.local_dir
        )
        .into());
    }

    let mut ssh_cmd = String::from("ssh");
    if let Some(port) = params.port {
        ssh_cmd.push_str(&format!(" -p {}", port));
    }
    if let Some(key_path) = params.identity_file {
        ssh_cmd.push_str(&format!(" -i \"{}\"", key_path.display()));
    }

    let mut cmd = Command::new("rsync");
    cmd.args(["-avz", "--progress"]);

    if params.dry_run {
        cmd.arg("--dry-run");
    }

    if params.delete {
        cmd.arg("--delete");
    }

    cmd.arg("-e").arg(ssh_cmd);

    let default_exclude = vec![".DS_Store".to_string()];
    let excludes_to_use = if params.excludes.is_empty() {
        &default_exclude[..]
    } else {
        params.excludes
    };

    for exclude in excludes_to_use {
        cmd.arg("--exclude").arg(exclude);
    }

    let formatted_local = if params.local_dir.ends_with('/') {
        params.local_dir.to_string()
    } else {
        format!("{}/", params.local_dir)
    };

    let remote_target = format!("{}@{}:{}", params.ssh_user, params.host, params.remote_dir);

    cmd.arg(formatted_local).arg(remote_target);
    cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    if params.dry_run {
        println!("rsync を実行中 (DRY-RUN モード)...");
    } else {
        println!("rsync を実行中...");
    }

    let status = cmd.status()?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("rsync が終了コード {:?} で失敗しました。", status.code()).into())
    }
}