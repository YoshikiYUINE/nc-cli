use clap::{Parser, Subcommand};
use serde::Deserialize;
use ssh2::Session;
use std::fs;
use std::io::Read;
use std::net::TcpStream;
use std::path::PathBuf;

/// config.toml の構造体定義
#[derive(Debug, Deserialize, Default, PartialEq)]
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

    /// SSH ログインユーザー名 (-s / --ssh-user)
    #[arg(short, long, global = true)]
    ssh_user: Option<String>,

    /// SSH 秘密鍵ファイルのパス (-i / --identity-file)
    #[arg(short, long, global = true)]
    identity_file: Option<PathBuf>,

    /// Nextcloud の occ コマンドパス
    #[arg(long, global = true)]
    occ_path: Option<String>,

    /// PHP 実行バイナリのパス
    #[arg(long, global = true)]
    php_path: Option<String>,

    /// 設定ファイルのパス (-c / --config-file)
    #[arg(short, long, default_value = "config.toml", global = true)]
    config_file: PathBuf,
}

#[derive(Subcommand, Debug, PartialEq)]
enum Commands {
    /// ユーザーを作成します (occ user:add)
    Add {
        /// 作成するユーザーID (uid)
        uid: String,

        /// パスワードを自動生成します
        #[arg(long)]
        generate_password: bool,

        /// 表示名
        #[arg(long)]
        display_name: Option<String>,

        /// 所属グループ
        #[arg(short, long)]
        group: Option<String>,

        /// メールアドレス
        #[arg(long)]
        email: Option<String>,
    },

    /// ユーザーの容量(Quota)を設定します (occ user:setting <uid> files quota <quota>)
    #[command(name = "set-quota")]
    SetQuota {
        /// 対象のユーザーID (uid)
        uid: String,

        /// 容量サイズ (例: "10 GB", "unlimited", "default")
        quota: String,
    },

    /// ユーザーを削除します (occ user:delete)
    #[command(name = "user-delete")]
    UserDelete {
        /// 削除対象の Nextcloud ユーザーID (例: okamura)
        #[arg(short, long)]
        target_user: String,
    },

    /// ファイルまたはディレクトリを削除します (occ files:delete)
    #[command(name = "file-delete")]
    FileDelete {
        /// 削除対象のファイルIDまたはパス
        path_or_id: String,
    },

    /// ユーザー一覧を取得します (occ user:list)
    #[command(name = "user-list")]
    UserList,

    /// グループ一覧を取得します (occ group:list)
    #[command(name = "group-list")]
    GroupList,

    /// 利用可能な OCC コマンド一覧を表示します (occ list)
    List,

    /// Nextcloud のステータス情報を表示します (occ status)
    Status,

    /// メンテナンスモードを切り替えます (occ maintenance:mode)
    #[command(name = "maintenance-mode")]
    MaintenanceMode {
        /// メンテナンスモードを有効化
        #[arg(long, conflicts_with = "off")]
        on: bool,

        /// メンテナンスモードを無効化
        #[arg(long, conflicts_with = "on")]
        off: bool,
    },
}

/// 各サブコマンドに応じた OCC リモート実行コマンド文字列を構築する関数
fn build_occ_command(command: &Commands, php_path: &str, occ_path: &str) -> String {
    match command {
        Commands::Add {
            uid,
            generate_password,
            display_name,
            group,
            email,
        } => {
            let mut cmd = format!("{} {} user:add {}", php_path, occ_path, uid);
            if *generate_password {
                cmd.push_str(" --generate-password");
            }
            if let Some(dn) = display_name {
                cmd.push_str(&format!(" --display-name=\"{}\"", dn));
            }
            if let Some(g) = group {
                cmd.push_str(&format!(" --group=\"{}\"", g));
            }
            if let Some(em) = email {
                cmd.push_str(&format!(" --email=\"{}\"", em));
            }
            cmd
        }
        Commands::SetQuota { uid, quota } => {
            format!(
                "{} {} user:setting {} files quota \"{}\"",
                php_path, occ_path, uid, quota
            )
        }
        Commands::UserDelete { target_user } => {
            format!(
                "{} {} user:delete {} --no-interaction --verbose",
                php_path, occ_path, target_user
            )
        }
        Commands::FileDelete { path_or_id } => {
            format!("{} {} files:delete \"{}\"", php_path, occ_path, path_or_id)
        }
        Commands::UserList => {
            format!("{} {} user:list --info --output=json", php_path, occ_path)
        }
        Commands::GroupList => {
            format!("{} {} group:list --info --output=json", php_path, occ_path)
        }
        Commands::List => {
            format!("{} {} list", php_path, occ_path)
        }
        Commands::Status => {
            format!("{} {} status --output=json", php_path, occ_path)
        }
        Commands::MaintenanceMode { on, off } => {
            if *on {
                format!("{} {} maintenance:mode --on", php_path, occ_path)
            } else if *off {
                format!("{} {} maintenance:mode --off", php_path, occ_path)
            } else {
                format!("{} {} maintenance:mode", php_path, occ_path)
            }
        }
    }
}

/// TOML 文字列を Config 構造体にパースするヘルパー関数
fn parse_config_toml(content: &str) -> Result<Config, toml::de::Error> {
    toml::from_str(content)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // 1. config.toml の読み込み
    let config: Config = if args.config_file.exists() {
        let content = fs::read_to_string(&args.config_file)?;
        parse_config_toml(&content)?
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
    let occ_command = build_occ_command(&args.command, &php_path, &occ_path);

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
        return Err(
            "SSH認証に失敗しました。秘密鍵、SSH Agent、またはユーザー名を確認してください。".into(),
        );
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
            Commands::Add { uid, .. } => {
                println!("成功: ユーザー '{}' を正常に作成しました。", uid);
            }
            Commands::SetQuota { uid, quota } => {
                println!(
                    "成功: ユーザー '{}' の容量制限を '{}' に設定しました。",
                    uid, quota
                );
            }
            Commands::UserDelete { target_user } => {
                println!("成功: ユーザー '{}' を正常に削除しました。", target_user);
            }
            Commands::FileDelete { path_or_id } => {
                println!(
                    "成功: ファイル/ディレクトリ '{}' を正常に削除しました。",
                    path_or_id
                );
            }
            Commands::UserList => {
                println!("成功: ユーザー一覧を取得しました。");
            }
            Commands::GroupList => {
                println!("成功: グループ一覧を取得しました。");
            }
            Commands::List => {
                println!("成功: OCCコマンド一覧を取得しました。");
            }
            Commands::Status => {
                println!("成功: ステータス情報を取得しました。");
            }
            Commands::MaintenanceMode { on, off } => {
                if *on {
                    println!("成功: メンテナンスモードを有効化しました。");
                } else if *off {
                    println!("成功: メンテナンスモードを無効化しました。");
                } else {
                    println!("成功: メンテナンスモードコマンドを実行しました。");
                }
            }
        }
    } else {
        eprintln!(
            "失敗: コマンドが終了コード {} で終了しました。",
            exit_status
        );
    }

    Ok(())
}

// -----------------------------------------------------------------------------
// Unit Tests
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    // --- 1. OCC コマンド文字列生成ロジックのテスト ---
    #[test]
    fn test_build_occ_command_add_full() {
        let cmd = Commands::Add {
            uid: "okamura".to_string(),
            generate_password: true,
            display_name: Some("岡村".to_string()),
            group: Some("admin".to_string()),
            email: Some("okamura@example.com".to_string()),
        };
        let result = build_occ_command(&cmd, "php", "./occ");
        assert_eq!(
            result,
            "php ./occ user:add okamura --generate-password --display-name=\"岡村\" --group=\"admin\" --email=\"okamura@example.com\""
        );
    }

    #[test]
    fn test_build_occ_command_add_minimal() {
        let cmd = Commands::Add {
            uid: "okamura".to_string(),
            generate_password: false,
            display_name: None,
            group: None,
            email: None,
        };
        let result = build_occ_command(&cmd, "php", "./occ");
        assert_eq!(result, "php ./occ user:add okamura");
    }

    #[test]
    fn test_build_occ_command_set_quota() {
        let cmd = Commands::SetQuota {
            uid: "okamura".to_string(),
            quota: "10 GB".to_string(),
        };
        let result = build_occ_command(&cmd, "php", "./occ");
        assert_eq!(
            result,
            "php ./occ user:setting okamura files quota \"10 GB\""
        );
    }

    #[test]
    fn test_build_occ_command_user_delete() {
        let cmd = Commands::UserDelete {
            target_user: "okamura".to_string(),
        };
        let result = build_occ_command(&cmd, "/usr/bin/php", "/var/www/nextcloud/occ");
        assert_eq!(
            result,
            "/usr/bin/php /var/www/nextcloud/occ user:delete okamura --no-interaction --verbose"
        );
    }

    #[test]
    fn test_build_occ_command_file_delete() {
        let cmd = Commands::FileDelete {
            path_or_id: "12345".to_string(),
        };
        let result = build_occ_command(&cmd, "php", "./occ");
        assert_eq!(result, "php ./occ files:delete \"12345\"");
    }

    #[test]
    fn test_build_occ_command_user_list() {
        let cmd = Commands::UserList;
        let result = build_occ_command(&cmd, "php", "./occ");
        assert_eq!(result, "php ./occ user:list --info --output=json");
    }

    #[test]
    fn test_build_occ_command_group_list() {
        let cmd = Commands::GroupList;
        let result = build_occ_command(&cmd, "php", "./occ");
        assert_eq!(result, "php ./occ group:list --info --output=json");
    }

    #[test]
    fn test_build_occ_command_list() {
        let cmd = Commands::List;
        let result = build_occ_command(&cmd, "php", "./occ");
        assert_eq!(result, "php ./occ list");
    }

    #[test]
    fn test_build_occ_command_status() {
        let cmd = Commands::Status;
        let result = build_occ_command(&cmd, "php", "./occ");
        assert_eq!(result, "php ./occ status --output=json");
    }

    #[test]
    fn test_build_occ_command_maintenance_mode() {
        let cmd_on = Commands::MaintenanceMode {
            on: true,
            off: false,
        };
        let result_on = build_occ_command(&cmd_on, "php", "./occ");
        assert_eq!(result_on, "php ./occ maintenance:mode --on");

        let cmd_off = Commands::MaintenanceMode {
            on: false,
            off: true,
        };
        let result_off = build_occ_command(&cmd_off, "php", "./occ");
        assert_eq!(result_off, "php ./occ maintenance:mode --off");
    }

    // --- 2. TOML 設定パースのテスト ---
    #[test]
    fn test_parse_config_toml_valid() {
        let toml_data = r#"
            host = "happy.com:22"
            ssh_user = "ubuntu"
            identity_file = "/home/ubuntu/.ssh/id_rsa"
            occ_path = "/var/www/nc/occ"
            php_path = "/usr/bin/php8.3"
        "#;

        let config = parse_config_toml(toml_data).unwrap();
        assert_eq!(config.host, Some("happy.com:22".to_string()));
        assert_eq!(config.ssh_user, Some("ubuntu".to_string()));
        assert_eq!(
            config.identity_file,
            Some(PathBuf::from("/home/ubuntu/.ssh/id_rsa"))
        );
        assert_eq!(config.occ_path, Some("/var/www/nc/occ".to_string()));
        assert_eq!(config.php_path, Some("/usr/bin/php8.3".to_string()));
    }

    #[test]
    fn test_parse_config_toml_partial() {
        let toml_data = r#"
            host = "happy.com:22"
            ssh_user = "ubuntu"
        "#;

        let config = parse_config_toml(toml_data).unwrap();
        assert_eq!(config.host, Some("happy.com:22".to_string()));
        assert_eq!(config.ssh_user, Some("ubuntu".to_string()));
        assert_eq!(config.identity_file, None);
        assert_eq!(config.occ_path, None);
        assert_eq!(config.php_path, None);
    }

    // --- 3. CLI コマンドライン引数パースのテスト (clap) ---
    #[test]
    fn test_cli_parse_add_subcommand() {
        let parsed = Args::try_parse_from([
            "app",
            "add",
            "okamura",
            "--generate-password",
            "--display-name",
            "岡村",
            "-g",
            "admin",
            "--email",
            "okamura@example.com",
        ])
        .unwrap();

        assert_eq!(
            parsed.command,
            Commands::Add {
                uid: "okamura".to_string(),
                generate_password: true,
                display_name: Some("岡村".to_string()),
                group: Some("admin".to_string()),
                email: Some("okamura@example.com".to_string()),
            }
        );
    }

    #[test]
    fn test_cli_parse_set_quota_subcommand() {
        let parsed = Args::try_parse_from(["app", "set-quota", "okamura", "10 GB"]).unwrap();
        assert_eq!(
            parsed.command,
            Commands::SetQuota {
                uid: "okamura".to_string(),
                quota: "10 GB".to_string(),
            }
        );
    }

    #[test]
    fn test_cli_parse_user_delete_subcommand() {
        let parsed = Args::try_parse_from(["app", "user-delete", "-t", "test_user"]).unwrap();
        assert_eq!(
            parsed.command,
            Commands::UserDelete {
                target_user: "test_user".to_string()
            }
        );
    }

    #[test]
    fn test_cli_parse_file_delete_subcommand() {
        let parsed = Args::try_parse_from(["app", "file-delete", "12345"]).unwrap();
        assert_eq!(
            parsed.command,
            Commands::FileDelete {
                path_or_id: "12345".to_string()
            }
        );
    }

    #[test]
    fn test_cli_parse_user_list_subcommand() {
        let parsed = Args::try_parse_from(["app", "user-list"]).unwrap();
        assert_eq!(parsed.command, Commands::UserList);
    }

    #[test]
    fn test_cli_parse_group_list_subcommand() {
        let parsed = Args::try_parse_from(["app", "group-list"]).unwrap();
        assert_eq!(parsed.command, Commands::GroupList);
    }

    #[test]
    fn test_cli_parse_global_flags() {
        let parsed = Args::try_parse_from([
            "app",
            "--host",
            "remote.host:2222",
            "-s",
            "myuser",
            "status",
        ])
        .unwrap();

        assert_eq!(parsed.command, Commands::Status);
        assert_eq!(parsed.host, Some("remote.host:2222".to_string()));
        assert_eq!(parsed.ssh_user, Some("myuser".to_string()));
    }

    #[test]
    fn test_cli_parse_maintenance_mode_subcommand() {
        let parsed_on = Args::try_parse_from(["app", "maintenance-mode", "--on"]).unwrap();
        assert_eq!(
            parsed_on.command,
            Commands::MaintenanceMode {
                on: true,
                off: false,
            }
        );

        let parsed_off = Args::try_parse_from(["app", "maintenance-mode", "--off"]).unwrap();
        assert_eq!(
            parsed_off.command,
            Commands::MaintenanceMode {
                on: false,
                off: true,
            }
        );
    }
}
