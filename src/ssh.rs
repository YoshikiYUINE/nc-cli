use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;
use std::path::PathBuf;

/// SSH接続に必要な設定パラメータ
pub struct SshConfig {
    pub host: String,
    pub ssh_user: String,
    pub identity_file: Option<PathBuf>,
}

/// SSHコマンド実行結果の保持構造体
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_status: i32,
}

/// SSHサーバーに接続し、リモートコマンドを実行する関数
pub fn execute_remote_command(
    config: &SshConfig,
    command: &str,
) -> Result<CommandResult, Box<dyn std::error::Error>> {
    println!("SSH接続を開始します: {}@{}", config.ssh_user, config.host);
    let tcp = TcpStream::connect(&config.host)?;
    let mut sess = Session::new()?;
    sess.set_tcp_stream(tcp);
    sess.handshake()?;

    if let Some(key_path) = &config.identity_file {
        println!("秘密鍵ファイルを使用して認証します: {:?}", key_path);
        sess.userauth_pubkey_file(&config.ssh_user, None, key_path, None)?;
    } else {
        println!("鍵パス未指定のため、SSH Agent 経由で認証します");
        sess.userauth_agent(&config.ssh_user)?;
    }

    if !sess.authenticated() {
        return Err(
            "SSH認証に失敗しました。秘密鍵、SSH Agent、またはユーザー名を確認してください。".into(),
        );
    }
    println!("SSH認証に成功しました。");

    println!("実行コマンド: {}", command);
    let mut channel = sess.channel_session()?;
    channel.exec(command)?;

    let mut stdout_buf = String::new();
    let mut stderr_buf = String::new();

    channel.read_to_string(&mut stdout_buf)?;
    channel.stderr().read_to_string(&mut stderr_buf)?;

    channel.wait_close()?;
    let exit_status = channel.exit_status()?;

    Ok(CommandResult {
        stdout: stdout_buf,
        stderr: stderr_buf,
        exit_status,
    })
}

// -----------------------------------------------------------------------------
// ユニットテストの概要コメント
// -----------------------------------------------------------------------------
// 注意: 実際のネットワーク接続を伴う SSH 処理のモックテストは単体テストでは難しいため、
// 通信層のロジックテストはプロジェクト直下の `tests/` ディレクトリ配下に
// 結合テスト（Integration Test）として記述するか、モックライブラリ（mockall等）の導入を検討します。