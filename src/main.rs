use clap::{Parser, Subcommand};
use serde::Deserialize;
use ssh2::Session;
use std::fs;
use std::io::Read;
use std::net::TcpStream;
use std::path::PathBuf;

/// config.toml の構造体定義
#[derive(Debug, Deserialize, Default)]
struct Config {
    host: Option<String>,
    ssh_user: Option<String>,
    identity_file: Option<PathBuf>,
    occ_path: Option<String>,
    php_path: Option<String>,
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Nextcloud 管理 CLI (SSH経由)")]
struct Args {
    #[command(subcommand)]
    command: Commands,

    /// SSH サーバーのホスト名とポート (例: example.server.jp:22)
    #[arg(long, global = true)]
    host: Option<String>,

    /// SSH ログインユーザー名 (例: ubuntu, root など)
    #[arg(short, long, global = true)]
    ssh_user: Option<String>,

    /// SSH 秘密鍵ファイルのパス (指定がない場合は SSH Agent を使用)
    #[arg(short, long, global = true)]
    identity_file: Option<PathBuf>,

    /// Nextcloud の occ コマンドパス
    #[arg(long, global = true)]
    occ_path: Option<String>,

    /// PHP 実行バイナリのパス
    #[arg(long, global = true)]
    php_path: Option<String>,

    /// 設定ファイルのパス
    #[arg(short, long, default_value = "config.toml", global = true)]
    config_file: PathBuf,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// ユーザーを削除します
    Delete {
        /// 削除対象の Nextcloud ユーザーID (例: okamura)
        #[arg(short, long)]
        target_user: String,
    },
    /// ユーザー一覧を取得します (JSON形式)
    List,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // 1. config.toml の読み込み
    let config: Config = if args.config_file.exists() {
        let content = fs::read_to_string(&args.config_file)?;
        toml::from_str(&content)?
    } else {
        Config::default()
    };

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
    let occ_command = match &args.command {
        Commands::Delete { target_user } => {
            format!(
                "{} {} user:delete {} --no-interaction --verbose",
                php_path, occ_path, target_user
            )
        }
        Commands::List => {
            format!(
                "{} {} user:list --info --output=json",
                php_path, occ_path
            )
        }
    };

    // 4. SSH 接続と認証
    println!("SSH接続を開始します: {}@{}", ssh_user, host);
    let tcp = TcpStream::connect(&host)?;
    let mut sess = Session::new()?;
    sess.set_tcp_stream(tcp);
    sess.handshake()?;

    if let Some(key_path) = &identity_file {
        println!("秘密鍵ファイルを使用して認証します: {:?}", key_path);
        sess.userauth_pubkey_file(&ssh_user, None, key_path, None)?;
    } else {
        println!("鍵パス未指定のため、SSH Agent 経由で認証します");
        sess.userauth_agent(&ssh_user)?;
    }

    if !sess.authenticated() {
        return Err("SSH認証に失敗しました。秘密鍵、SSH Agent、またはユーザー名を確認してください。".into());
    }
    println!("SSH認証に成功しました。");

    // 5. リモートコマンドの実行
    println!("実行コマンド: {}", occ_command);
    let mut channel = sess.channel_session()?;
    channel.exec(&occ_command)?;

    let mut stdout_buf = String::new();
    let mut stderr_buf = String::new();

    channel.read_to_string(&mut stdout_buf)?;
    channel.stderr().read_to_string(&mut stderr_buf)?;

    channel.wait_close()?;
    let exit_status = channel.exit_status()?;

    // 6. 出力と結果判定
    if !stdout_buf.is_empty() {
        println!("\n--- [標準出力] ---\n{}", stdout_buf.trim());
    }
    if !stderr_buf.is_empty() {
        eprintln!("\n--- [標準エラー出力] ---\n{}", stderr_buf.trim());
    }

    println!("\n----------------------------------------");
    if exit_status == 0 {
        match &args.command {
            Commands::Delete { target_user } => {
                println!("成功: ユーザー '{}' を正常に削除しました。", target_user);
            }
            Commands::List => {
                println!("成功: ユーザー一覧を取得しました。");
            }
        }
    } else {
        eprintln!("失敗: コマンドが終了コード {} で終了しました。", exit_status);
    }

    Ok(())
}