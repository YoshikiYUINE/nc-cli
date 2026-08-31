pub mod cli;
pub mod config;
pub mod occ;
pub mod ssh;
pub mod sync;

use crate::cli::{Args, Commands};
use crate::config::Config;
use crate::ssh::{execute_remote_command, SshConfig};
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