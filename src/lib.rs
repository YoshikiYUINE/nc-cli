pub mod cli;
pub mod config;
pub mod occ;
pub mod ssh;
pub mod sync;

use crate::cli::{Args, Commands};
use crate::config::Config;
use crate::ssh::{execute_remote_command, execute_remote_command_bytes, SshConfig};
use crate::sync::{run_rsync, SyncParams};

/// アプリケーションメインロジックの統括実行関数
pub fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    // 1. config.toml の読み込み
    let config = Config::load(&args.config_file)?;

    // 2. 共通設定値の決定 (CLI引数 > config.toml > デフォルト値)
    let host_raw = args
        .host
        .or(config.host)
        .unwrap_or_else(|| "example.server.jp:22".to_string());

    let (host, parsed_port) = {
        let tmp_cfg = Config {
            host: Some(host_raw),
            ..Default::default()
        };
        tmp_cfg.get_host_and_port()
    };

    let ssh_user = args.ssh_user.or(config.ssh_user).ok_or(
        "エラー: SSH ユーザー名が未指定です。CLI引数 (--ssh-user) または config.toml で指定してください。",
    )?;

    let identity_file = args.identity_file.or(config.identity_file);

    let occ_path = args
        .occ_path
        .or(config.occ_path)
        .unwrap_or_else(|| "./nextcloud/occ".to_string());

    let php_path = args
        .php_path
        .or(config.php_path)
        .unwrap_or_else(|| "/opt/php-8.3/bin/php".to_string());

    match &args.command {
        Commands::Sync {
            remote_dir,
            exclude,
            delete,
            dry_run,
            scan_all,
            scan_user,
            scan_path,
        } => {
            // ローカル側同期元フォルダー（srcと同じ階層の rsync_target）
            let local_dir = "rsync_target";

            // 1. rsync による同期を実行
            let sync_params = SyncParams {
                local_dir,
                remote_dir,
                host: &host,
                port: parsed_port,
                ssh_user: &ssh_user,
                identity_file: identity_file.as_ref(),
                excludes: exclude,
                delete: *delete,
                dry_run: *dry_run,
            };

            run_rsync(&sync_params)?;

            // ドライランの場合は安全のため files:scan をスキップして終了
            if *dry_run {
                println!("\n[DRY RUN] 試行運転が終了しました。実際のファイル同期および Nextcloud スキャンはスキップされました。");
                println!("----------------------------------------");
                args.command.print_success();
                return Ok(());
            }

            println!("\nローカルファイルの同期が完了しました。");

            // 2. 必要に応じて Nextcloud の file:scan を実行
            if *scan_all || scan_user.is_some() || scan_path.is_some() {
                println!("Nextcloud のファイルスキャンを実行中...");
                let scan_cmd = Commands::FileScan {
                    user_id: scan_user.clone(),
                    all: *scan_all,
                    path: scan_path.clone(),
                    unscanned: false,
                    shallow: false,
                    home_only: false,
                    no_lock: false,
                };

                let occ_command = occ::build_occ_command(&scan_cmd, &php_path, &occ_path);
                let host_with_port = if let Some(p) = parsed_port {
                    format!("{}:{}", host, p)
                } else {
                    host.clone()
                };

                let ssh_config = SshConfig {
                    host: host_with_port,
                    ssh_user: ssh_user.clone(),
                    identity_file: identity_file.clone(),
                };

                let result = execute_remote_command(&ssh_config, &occ_command)?;
                if !result.stdout.is_empty() {
                    println!("\n--- [標準出力] ---\n{}", result.stdout.trim());
                }
                if !result.stderr.is_empty() {
                    eprintln!("\n--- [標準エラー出力] ---\n{}", result.stderr.trim());
                }
            }
            println!("\n----------------------------------------");
            args.command.print_success();
        }
        Commands::FileGet { path_or_id, .. } => {
            let host_with_port = if let Some(p) = parsed_port {
                format!("{}:{}", host, p)
            } else {
                host
            };

            let is_stdout = args.command.is_file_get_stdout();
            let is_numeric_id = path_or_id.chars().all(|c| c.is_ascii_digit());

            // 1. ファイル名が未指定かつファイルID（数字のみ）の場合、Nextcloud から実際のファイル名を取得する
            let mut resolved_filename = args.command.get_file_get_filename();
            if !is_stdout && resolved_filename.as_deref() == Some(path_or_id.as_str()) && is_numeric_id {
                let info_cmd = occ::build_info_file_command(&php_path, &occ_path, path_or_id);
                let ssh_config = SshConfig {
                    host: host_with_port.clone(),
                    ssh_user: ssh_user.clone(),
                    identity_file: identity_file.clone(),
                };
                // info:file コマンドを実行してファイル名を解決
                if let Ok(info_result) = execute_remote_command_bytes(&ssh_config, &info_cmd, true) {
                    if info_result.exit_status == 0 {
                        let info_text = String::from_utf8_lossy(&info_result.stdout);
                        if let Some(real_name) = occ::parse_filename_from_info_output(&info_text) {
                            resolved_filename = Some(real_name);
                        }
                    }
                }
            }

            // 2. occ files:get コマンドを実行
            let occ_command = occ::build_occ_command(&args.command, &php_path, &occ_path);
            let ssh_config = SshConfig {
                host: host_with_port,
                ssh_user,
                identity_file,
            };

            let result = execute_remote_command_bytes(&ssh_config, &occ_command, is_stdout)?;

            if result.exit_status != 0 {
                let err_msg = if !result.stderr.trim().is_empty() {
                    result.stderr.trim().to_string()
                } else if !result.stdout.is_empty() {
                    String::from_utf8_lossy(&result.stdout).trim().to_string()
                } else {
                    format!("終了コード: {}", result.exit_status)
                };
                return Err(
                    format!("ファイル '{}' の取得に失敗しました: {}", path_or_id, err_msg).into(),
                );
            }

            // 3. PHP CLI の警告メッセージ（PCNTL や PHP Warning 等）をファイルデータから除去
            let (clean_stdout, warnings) = occ::strip_php_cli_warnings(&result.stdout);
            for w in &warnings {
                eprintln!("[サーバー警告] {}", w);
            }

            if is_stdout {
                use std::io::Write;
                std::io::stdout().write_all(clean_stdout)?;
                std::io::stdout().flush()?;
            } else {
                let filename = resolved_filename.unwrap_or_else(|| "downloaded_file".to_string());

                let downloads_dir = std::path::Path::new("downloads");
                if !downloads_dir.exists() {
                    std::fs::create_dir_all(downloads_dir)?;
                }

                let target_path = downloads_dir.join(&filename);
                std::fs::write(&target_path, clean_stdout)?;

                if !result.stderr.is_empty() {
                    eprintln!("\n--- [標準エラー出力] ---\n{}", result.stderr.trim());
                }

                println!("\n----------------------------------------");
                println!(
                    "ファイル '{}' を 'downloads/{}' にダウンロードしました。(サイズ: {} バイト)",
                    path_or_id,
                    filename,
                    clean_stdout.len()
                );
            }
        }
        Commands::FileInfo { path_or_id, raw } => {
            let host_with_port = if let Some(p) = parsed_port {
                format!("{}:{}", host, p)
            } else {
                host
            };

            let ssh_config = SshConfig {
                host: host_with_port,
                ssh_user,
                identity_file,
            };

            if *raw {
                let output_str = get_file_info(&ssh_config, &php_path, &occ_path, path_or_id)?;
                println!("{}", output_str);
            } else {
                let occ_command = occ::build_occ_command(&args.command, &php_path, &occ_path);
                let result = execute_remote_command(&ssh_config, &occ_command)?;

                if !result.stdout.is_empty() {
                    println!("\n--- [標準出力] ---\n{}", result.stdout.trim());
                }
                if !result.stderr.is_empty() {
                    eprintln!("\n--- [標準エラー出力] ---\n{}", result.stderr.trim());
                }

                println!("\n----------------------------------------");
                if result.exit_status == 0 {
                    args.command.print_success();
                } else {
                    eprintln!(
                        "失敗: コマンドが終了コード {} で終了しました。",
                        result.exit_status
                    );
                }
            }
        }
        _ => {
            // 既存の OCC コマンド実行
            let host_with_port = if let Some(p) = parsed_port {
                format!("{}:{}", host, p)
            } else {
                host
            };

            let occ_command = occ::build_occ_command(&args.command, &php_path, &occ_path);
            let ssh_config = SshConfig {
                host: host_with_port,
                ssh_user,
                identity_file,
            };

            let result = execute_remote_command(&ssh_config, &occ_command)?;

            if !result.stdout.is_empty() {
                println!("\n--- [標準出力] ---\n{}", result.stdout.trim());
            }
            if !result.stderr.is_empty() {
                eprintln!("\n--- [標準エラー出力] ---\n{}", result.stderr.trim());
            }

            println!("\n----------------------------------------");
            if result.exit_status == 0 {
                args.command.print_success();
            } else {
                eprintln!(
                    "失敗: コマンドが終了コード {} で終了しました。",
                    result.exit_status
                );
            }
        }
    }

    Ok(())
}

/// Nextcloud 上のファイル情報を取得する関数
pub fn get_file_info(
    ssh_config: &SshConfig,
    php_path: &str,
    occ_path: &str,
    path_or_id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let occ_command = occ::build_info_file_command(php_path, occ_path, path_or_id);
    let result = execute_remote_command_bytes(ssh_config, &occ_command, true)?;

    if result.exit_status != 0 {
        let err_msg = if !result.stderr.trim().is_empty() {
            result.stderr.trim().to_string()
        } else if !result.stdout.is_empty() {
            String::from_utf8_lossy(&result.stdout).trim().to_string()
        } else {
            format!("終了コード: {}", result.exit_status)
        };
        return Err(format!("ファイル情報の取得に失敗しました: {}", err_msg).into());
    }

    // PHP CLI の警告メッセージ（PCNTL や PHP Warning 等）を除去
    let (clean_stdout, warnings) = occ::strip_php_cli_warnings(&result.stdout);
    for w in &warnings {
        eprintln!("[サーバー警告] {}", w);
    }

    let output_str = String::from_utf8_lossy(clean_stdout).trim().to_string();
    Ok(output_str)
}
