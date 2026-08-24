use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// CLIコマンドライン引数の全体定義
#[derive(Parser, Debug)]
#[command(author, version, about = "Nextcloud 管理 CLI (SSH経由)")]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,

    /// SSH サーバーのホスト名とポート (例: example.server.jp:22)
    #[arg(long, global = true)]
    pub host: Option<String>,

    /// SSH ログインユーザー名 (-s / --ssh-user)
    #[arg(short, long, global = true)]
    pub ssh_user: Option<String>,

    /// SSH 秘密鍵ファイルのパス (-i / --identity-file)
    #[arg(short, long, global = true)]
    pub identity_file: Option<PathBuf>,

    /// Nextcloud の occ コマンドパス
    #[arg(long, global = true)]
    pub occ_path: Option<String>,

    /// PHP 実行バイナリのパス
    #[arg(long, global = true)]
    pub php_path: Option<String>,

    /// 設定ファイルのパス (-c / --config-file)
    #[arg(short, long, default_value = "config.toml", global = true)]
    pub config_file: PathBuf,
}

/// 各サブコマンドの定義
#[derive(Subcommand, Debug, PartialEq)]
pub enum Commands {
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

impl Commands {
    /// コマンド成功時のメッセージを出力する helper 関数
    pub fn print_success(&self) {
        match self {
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
    }
}

// -----------------------------------------------------------------------------
// CLI 引数パースのユニットテスト
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

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