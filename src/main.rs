use clap::Parser;
use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about = "Nextcloud ユーザー削除 CLI (SSH経由)")]
struct Args {
    /// 削除対象の Nextcloud ユーザーID (例: okamura)
    #[arg(short, long)]
    target_user: String,

    /// SSH サーバーのホスト名とポート (例: example.server.jp:22)
    #[arg(long, default_value = "example.server.jp:22")]
    host: String,

    /// SSH ログインユーザー名 (例: ubuntu, root など)
    #[arg(short, long)]
    ssh_user: String,

    /// SSH 秘密鍵ファイルのパス (例: ~/.ssh/id_rsa)
    //  Option にすることで省略可能（オプショナル）にする
    #[arg(short, long)]
    identity_file: Option<PathBuf>,

    /// Nextcloud の occ コマンドパス
    #[arg(long, default_value = "./nextcloud/occ")]
    occ_path: String,

    /// PHP 実行バイナリのパス
    #[arg(long, default_value = "/opt/php-8.3/bin/php")]
    php_path: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("SSH接続を開始します: {}@{}", args.ssh_user, args.host);

    // 1. TCP 接続の確立
    let tcp = TcpStream::connect(&args.host)?;
    let mut sess = Session::new()?;
    sess.set_tcp_stream(tcp);
    sess.handshake()?;

    // 2. SSH 公開鍵認証
    if let Some(key_path) = &args.identity_file {
        // --identity-file が渡された場合は指定のファイルで認証
        sess.userauth_pubkey_file(&args.ssh_user, None, key_path, None)?;
    } else {
        // 指定がない場合は SSH Agent 経由で認証（Dev Container 環境向け）
        sess.userauth_agent(&args.ssh_user)?;
    }
    if !sess.authenticated() {
        return Err("SSH認証に失敗しました。秘密鍵またはユーザー名を確認してください。".into());
    }
    println!("SSH認証に成功しました。");

    // 3. occ user:delete コマンドの構築
    // ※ --non-interactive (または -n) を指定して対話型確認 (Y/n) をスキップします
    let command = format!(
        "{} {} user:delete {} --non-interactive",
        args.php_path, args.occ_path, args.target_user
    );

    println!("実行コマンド: {}", command);

    // 4. SSH チャネルを開いてリモートコマンドを実行
    let mut channel = sess.channel_session()?;
    channel.exec(&command)?;

    // 5. 標準出力 (STDOUT) と標準エラー出力 (STDERR) の受け取り
    let mut stdout_buf = String::new();
    let mut stderr_buf = String::new();

    channel.read_to_string(&mut stdout_buf)?;
    channel.stderr().read_to_string(&mut stderr_buf)?;

    channel.wait_close()?;
    let exit_status = channel.exit_status()?;

    // 6. 実行結果の出力と判定
    if !stdout_buf.is_empty() {
        println!("\n--- [標準出力] ---\n{}", stdout_buf.trim());
    }
    if !stderr_buf.is_empty() {
        eprintln!("\n--- [標準エラー出力] ---\n{}", stderr_buf.trim());
    }

    println!("\n----------------------------------------");
    if exit_status == 0 {
        println!(
            "成功: ユーザー '{}' を正常に削除しました。",
            args.target_user
        );
    } else {
        eprintln!(
            "失敗: コマンドが終了コード {} で終了しました。",
            exit_status
        );
    }

    Ok(())
}
