use crate::cli::Commands;

/// 各サブコマンドに応じた OCC リモート実行コマンド文字列を構築する関数
pub fn build_occ_command(command: &Commands, php_path: &str, occ_path: &str) -> String {
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
        Commands::FilePut {
            local_path,
            target_path_or_id,
        } => {
            format!(
                "{} {} files:put \"{}\" \"{}\"",
                php_path, occ_path, local_path, target_path_or_id
            )
        }
        Commands::FileScan {
            user_id,
            all,
            path,
            unscanned,
            shallow,
            home_only,
            no_lock,
        } => {
            let mut cmd = format!("{} {} files:scan --output=json", php_path, occ_path);
            if *all {
                cmd.push_str(" --all");
            }
            if let Some(p) = path {
                cmd.push_str(&format!(" --path=\"{}\"", p));
            }
            if let Some(u) = user_id {
                cmd.push_str(&format!(" {}", u));
            }
            if *unscanned {
                cmd.push_str(" --unscanned");
            }
            if *shallow {
                cmd.push_str(" --shallow");
            }
            if *home_only {
                cmd.push_str(" --home-only");
            }
            if *no_lock {
                cmd.push_str(" --no-lock");
            }
            cmd
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

// -----------------------------------------------------------------------------
// OCC コマンド文字列生成ロジックのユニットテスト
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_build_occ_command_file_put() {
        let cmd = Commands::FilePut {
            local_path: "/tmp/sample.txt".to_string(),
            target_path_or_id: "admin/files/sample.txt".to_string(),
        };
        let result = build_occ_command(&cmd, "php", "./occ");
        assert_eq!(
            result,
            "php ./occ files:put \"/tmp/sample.txt\" \"admin/files/sample.txt\""
        );
    }

    #[test]
    fn test_build_occ_command_file_scan() {
        // --all + 各種オプション
        let cmd_all = Commands::FileScan {
            user_id: None,
            all: true,
            path: None,
            unscanned: true,
            shallow: true,
            home_only: true,
            no_lock: true,
        };
        let result_all = build_occ_command(&cmd_all, "php", "./occ");
        assert_eq!(
            result_all,
            "php ./occ files:scan --output=json --all --unscanned --shallow --home-only --no-lock"
        );

        // ユーザー指定
        let cmd_user = Commands::FileScan {
            user_id: Some("admin".to_string()),
            all: false,
            path: None,
            unscanned: false,
            shallow: false,
            home_only: false,
            no_lock: false,
        };
        let result_user = build_occ_command(&cmd_user, "php", "./occ");
        assert_eq!(result_user, "php ./occ files:scan --output=json admin");

        // パス指定 + --unscanned
        let cmd_path = Commands::FileScan {
            user_id: None,
            all: false,
            path: Some("admin/files/Documents".to_string()),
            unscanned: true,
            shallow: false,
            home_only: false,
            no_lock: false,
        };
        let result_path = build_occ_command(&cmd_path, "php", "./occ");
        assert_eq!(
            result_path,
            "php ./occ files:scan --output=json --path=\"admin/files/Documents\" --unscanned"
        );
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
}