use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// CLI 引数定義
#[derive(Parser, Debug)]
#[command(author, version, about = "Nextcloud 管理 CLI (SSH 経由)")]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,

    /// SSH ホスト (形式: example.server.jp:22)
    #[arg(long, global = true)]
    pub host: Option<String>,

    /// SSH ユーザー名 (-s / --ssh-user)
    #[arg(short, long, global = true)]
    pub ssh_user: Option<String>,

    /// SSH 秘密鍵パス (-i / --identity-file)
    #[arg(short, long, global = true)]
    pub identity_file: Option<PathBuf>,

    /// Nextcloud の occ パス
    #[arg(long, global = true)]
    pub occ_path: Option<String>,

    /// PHP 実行パス
    #[arg(long, global = true)]
    pub php_path: Option<String>,

    /// 設定ファイル (-c / --config-file)
    #[arg(short, long, default_value = "config.toml", global = true)]
    pub config_file: PathBuf,
}

/// サブコマンド定義
#[derive(Subcommand, Debug, PartialEq)]
pub enum Commands {
    /// ユーザー追加 (occ user:add)
    Add {
        /// ユーザー ID (uid)
        uid: String,

        /// パスワード自動生成
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

    /// クォータ (Quota) 設定 (occ user:setting <uid> files quota <quota>)
    #[command(name = "set-quota")]
    SetQuota {
        /// ユーザー ID (uid)
        uid: String,

        /// 容量 (例: "10 GB", "unlimited", "default")
        quota: String,
    },

    /// ユーザー削除 (occ user:delete)
    #[command(name = "user-delete")]
    UserDelete {
        /// 削除対象の Nextcloud ユーザー ID (例: okamura)
        #[arg(short, long)]
        target_user: String,
    },

    /// ファイル削除 (occ files:delete)
    #[command(name = "file-delete")]
    FileDelete {
        /// ファイルパスまたは ID
        path_or_id: String,
    },

    /// ファイル配置 (occ files:put)
    #[command(name = "file-put", alias = "files-put")]
    FilePut {
        /// ローカルファイルパス (標準入力から読む場合は "-")
        local_path: String,

        /// Nextcloud 上の配置先相対パス
        target_path_or_id: String,
    },

    /// ファイルスキャン (occ files:scan)
    #[command(name = "file-scan", alias = "files-scan")]
    FileScan {
        /// ユーザー ID (指定時、特定ユーザーのみ)
        #[arg(conflicts_with_all = &["all", "path"])]
        user_id: Option<String>,

        /// 全ユーザーをスキャン
        #[arg(long, conflicts_with_all = &["user_id", "path"])]
        all: bool,

        /// パス指定 (例: "admin/files/Documents")
        #[arg(short, long, conflicts_with_all = &["user_id", "all"])]
        path: Option<String>,

        /// 未スキャンファイルのみ (--unscanned)
        #[arg(long)]
        unscanned: bool,

        /// 浅いスキャン (--shallow)
        #[arg(long)]
        shallow: bool,

        /// ホームディレクトリのみ (--home-only)
        #[arg(long)]
        home_only: bool,

        /// ロックなし (--no-lock)
        #[arg(long)]
        no_lock: bool,
    },

    /// ユーザー一覧表示 (occ user:list)
    #[command(name = "user-list")]
    UserList,

    /// グループ一覧表示 (occ group:list)
    #[command(name = "group-list")]
    GroupList,

    /// 利用可能な OCC コマンド一覧表示 (occ list)
    List,

    /// Nextcloud ステータス確認 (occ status)
    Status,

    /// メンテナンスモード設定 (occ maintenance:mode)
    #[command(name = "maintenance-mode")]
    MaintenanceMode {
        /// 有効化
        #[arg(long, conflicts_with = "off")]
        on: bool,

        /// 無効化
        #[arg(long, conflicts_with = "on")]
        off: bool,
    },

    /// rsync によるファイル同期 (rsync_target -> リモート) と Nextcloud への反映 (files:scan)
    #[command(name = "sync")]
    Sync {
        /// 同期先リモートディレクトリパス
        remote_dir: String,

        /// 除外パターン (複数指定可能)
        #[arg(short, long)]
        exclude: Vec<String>,

        /// 転送元に存在しないファイルを転送先から削除 (--delete)
        #[arg(long)]
        delete: bool,

        /// 試行運転を行う (実際には転送・削除しない) (-n / --dry-run)
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// 全ユーザーのファイルをスキャン (--all)
        #[arg(long, conflicts_with_all = &["scan_user", "scan_path"])]
        scan_all: bool,

        /// 特定ユーザーのファイルをスキャン ([USER_ID])
        #[arg(long, conflicts_with_all = &["scan_all", "scan_path"])]
        scan_user: Option<String>,

        /// 特定パスのファイルをスキャン (--path)
        #[arg(long, conflicts_with_all = &["scan_all", "scan_user"])]
        scan_path: Option<String>,
    },
}

impl Commands {
    /// 実行成功メッセージの helper
    pub fn print_success(&self) {
        match self {
            Commands::Add { uid, .. } => {
                println!("ユーザー '{}' を作成しました。", uid);
            }
            Commands::SetQuota { uid, quota } => {
                println!(
                    "ユーザー '{}' のクォータを '{}' に設定しました。",
                    uid, quota
                );
            }
            Commands::UserDelete { target_user } => {
                println!("ユーザー '{}' を削除しました。", target_user);
            }
            Commands::FileDelete { path_or_id } => {
                println!(
                    "ファイル/ディレクトリ '{}' を削除しました。",
                    path_or_id
                );
            }
            Commands::FilePut {
                local_path,
                target_path_or_id,
            } => {
                println!(
                    "ファイル '{}' を Nextcloud 上の '{}' に配置しました。",
                    local_path, target_path_or_id
                );
            }
            Commands::FileScan {
                user_id,
                all,
                path,
                ..
            } => {
                if *all {
                    println!("全ユーザーのファイルスキャンが完了しました。");
                } else if let Some(p) = path {
                    println!("パス '{}' のファイルスキャンが完了しました。", p);
                } else if let Some(u) = user_id {
                    println!("ユーザー '{}' のファイルスキャンが完了しました。", u);
                } else {
                    println!("ファイルスキャンが完了しました。");
                }
            }
            Commands::UserList => {
                println!("ユーザー一覧を取得しました。");
            }
            Commands::GroupList => {
                println!("グループ一覧を取得しました。");
            }
            Commands::List => {
                println!("利用可能な OCC コマンド一覧を取得しました。");
            }
            Commands::Status => {
                println!("ステータス情報を取得しました。");
            }
            Commands::MaintenanceMode { on, off } => {
                if *on {
                    println!("メンテナンスモードを有効にしました。");
                } else if *off {
                    println!("メンテナンスモードを無効にしました。");
                } else {
                    println!("メンテナンスモードの状態を取得しました。");
                }
            }
            Commands::Sync { dry_run, .. } => {
                if *dry_run {
                    println!("ドライランが完了しました。(変更は行われていません)");
                } else {
                    println!("ファイル同期および Nextcloud への反映処理が完了しました。");
                }
            }
        }
    }
}

// -----------------------------------------------------------------------------
// CLI 単体テスト
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_sync_subcommand() {
        let parsed = Args::try_parse_from([
            "app",
            "sync",
            "/remote/target",
            "-n",
            "--delete",
            "--scan-user",
            "admin",
        ])
        .unwrap();

        assert_eq!(
            parsed.command,
            Commands::Sync {
                remote_dir: "/remote/target".to_string(),
                exclude: vec![],
                delete: true,
                dry_run: true,
                scan_all: false,
                scan_user: Some("admin".to_string()),
                scan_path: None,
            }
        );
    }
}