pub mod cli;
pub mod config;
pub mod occ;
pub mod ssh;

use crate::cli::Args;
use crate::config::Config;
use crate::ssh::{execute_remote_command, SshConfig};

/// アプリケーションメインロジックの統括実行関数
pub fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    // 1. config.toml の読み込み
    let config = Config::load(&args.config_file)?;

    // 2. 共通設定値の決定 (CLI引数 > config.toml > デフォルト値)
    let host = args
        .host
        .or(config.host)
        .unwrap_or_else(|| "example.server.jp:22".to_string());

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

    // 3. サブコマンドに応じた occ コマンドの構築
    let occ_command = occ::build_occ_command(&args.command, &php_path, &occ_path);

    // 4. SSH 接続と実行
    let ssh_config = SshConfig {
        host,
        ssh_user,
        identity_file,
    };
    let result = execute_remote_command(&ssh_config, &occ_command)?;

    // 5. 出力と結果判定
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

    Ok(())
}