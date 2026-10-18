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
        Commands::FileGet { path_or_id, .. } => {
            format!("{} {} files:get \"{}\" -", php_path, occ_path, path_or_id)
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
            format!("{} {} list --output=json", php_path, occ_path)
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
        Commands::Sync { .. } => String::new(),
    }
}

/// info:file リモート実行コマンド文字列を構築する関数
pub fn build_info_file_command(php_path: &str, occ_path: &str, path_or_id: &str) -> String {
    format!("{} {} info:file \"{}\"", php_path, occ_path, path_or_id)
}

/// PHP CLI 実行時に標準出力の先頭に含まれる可能性のある警告メッセージ（PCNTL や PHP Warning 等）を除去する
pub fn strip_php_cli_warnings(raw: &[u8]) -> (&[u8], Vec<String>) {
    let mut offset = 0;
    let mut warnings = Vec::new();

    while offset < raw.len() {
        // 次の改行 (\n) を探す
        let next_newline = raw[offset..].iter().position(|&b| b == b'\n');
        let line_end = match next_newline {
            Some(pos) => offset + pos,
            None => raw.len(),
        };

        let mut line_bytes = &raw[offset..line_end];
        // 末尾の \r を除外
        if let Some(&b'\r') = line_bytes.last() {
            line_bytes = &line_bytes[..line_bytes.len() - 1];
        }

        // 行が UTF-8 文字列として警告パターンに一致するかチェック
        if let Ok(line_str) = std::str::from_utf8(line_bytes) {
            let trimmed = line_str.trim();
            if is_php_warning_line(trimmed) {
                if !trimmed.is_empty() {
                    warnings.push(trimmed.to_string());
                }
                // 改行文字分スキップ
                offset = if line_end < raw.len() {
                    line_end + 1
                } else {
                    raw.len()
                };
                continue;
            }
        }

        // 警告行でなければループ終了
        break;
    }

    (&raw[offset..], warnings)
}

/// 対象行が PHP / Nextcloud の警告メッセージかどうかを判定する
pub fn is_php_warning_line(line: &str) -> bool {
    line.starts_with("The process control (PCNTL) extensions are required")
        || line.starts_with("Additionally the function 'pcntl_signal'")
        || line.starts_with("PHP Warning:")
        || line.starts_with("PHP Notice:")
        || line.starts_with("PHP Deprecated:")
        || line.starts_with("PHP Strict Standards:")
        || line.starts_with("Xdebug:")
}

/// occ info:file の出力からファイル名を取得する
pub fn parse_filename_from_info_output(output: &str) -> Option<String> {
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || is_php_warning_line(trimmed) {
            continue;
        }
        // エラー出力等の場合は失敗とみなす
        if trimmed.starts_with("<error>")
            || (trimmed.starts_with("file ") && trimmed.ends_with("not found"))
        {
            return None;
        }
        // パス区切り文字を含まないファイル名の場合
        if !trimmed.contains('/') && !trimmed.contains('\\') {
            return Some(trimmed.to_string());
        } else {
            // パスが含まれている場合はファイル名部分を抽出
            let filename = std::path::Path::new(trimmed)
                .file_name()
                .and_then(|n| n.to_str());
            if let Some(f) = filename {
                return Some(f.to_string());
            }
        }
    }
    None
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
    fn test_build_occ_command_file_get_id() {
        let cmd = Commands::FileGet {
            path_or_id: "12345".to_string(),
            destination: None,
            output: None,
            stdout: false,
        };
        let result = build_occ_command(&cmd, "php", "./occ");
        assert_eq!(result, "php ./occ files:get \"12345\" -");
    }

    #[test]
    fn test_build_occ_command_file_get_path() {
        let cmd = Commands::FileGet {
            path_or_id: "/layla/files/Documents/report.pdf".to_string(),
            destination: None,
            output: None,
            stdout: true,
        };
        let result = build_occ_command(&cmd, "/usr/bin/php", "/var/www/nextcloud/occ");
        assert_eq!(
            result,
            "/usr/bin/php /var/www/nextcloud/occ files:get \"/layla/files/Documents/report.pdf\" -"
        );
    }

    #[test]
    fn test_build_occ_command_file_scan() {
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
        assert_eq!(result, "php ./occ list --output=json");
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

    #[test]
    fn test_build_info_file_command() {
        let result = build_info_file_command("php", "./occ", "12345");
        assert_eq!(result, "php ./occ info:file \"12345\"");
    }

    #[test]
    fn test_strip_php_cli_warnings_pcntl() {
        let mut raw = Vec::new();
        raw.extend_from_slice(b"The process control (PCNTL) extensions are required in case you want to interrupt long running commands - see https://www.php.net/manual/en/book.pcntl.php\n");
        raw.extend_from_slice(b"Additionally the function 'pcntl_signal' and 'pcntl_signal_dispatch' need to be enabled in your php.ini.\n");
        raw.extend_from_slice(b"%PDF-1.4 binary content\nsecond line");

        let (clean, warnings) = strip_php_cli_warnings(&raw);
        assert_eq!(clean, b"%PDF-1.4 binary content\nsecond line");
        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].starts_with("The process control"));
        assert!(warnings[1].starts_with("Additionally the function"));
    }

    #[test]
    fn test_strip_php_cli_warnings_crlf() {
        let mut raw = Vec::new();
        raw.extend_from_slice(b"The process control (PCNTL) extensions are required in case you want to interrupt long running commands - see https://www.php.net/manual/en/book.pcntl.php\r\n");
        raw.extend_from_slice(b"Additionally the function 'pcntl_signal' and 'pcntl_signal_dispatch' need to be enabled in your php.ini.\r\n");
        raw.extend_from_slice(b"Hello world\r\n");

        let (clean, warnings) = strip_php_cli_warnings(&raw);
        assert_eq!(clean, b"Hello world\r\n");
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn test_strip_php_cli_warnings_clean_content() {
        let raw = b"Clean content\nSecond line\n";
        let (clean, warnings) = strip_php_cli_warnings(raw);
        assert_eq!(clean, raw);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_parse_filename_from_info_output_with_warnings() {
        let output = r#"The process control (PCNTL) extensions are required in case you want to interrupt long running commands - see https://www.php.net/manual/en/book.pcntl.php
Additionally the function 'pcntl_signal' and 'pcntl_signal_dispatch' need to be enabled in your php.ini.
report_2026.pdf
  fileid: 12345
  mimetype: application/pdf
"#;
        let filename = parse_filename_from_info_output(output);
        assert_eq!(filename, Some("report_2026.pdf".to_string()));
    }

    #[test]
    fn test_parse_filename_from_info_output_with_path() {
        let output = r#"/layla/files/Documents/important.docx
  fileid: 67890
"#;
        let filename = parse_filename_from_info_output(output);
        assert_eq!(filename, Some("important.docx".to_string()));
    }

    #[test]
    fn test_parse_filename_from_info_output_error() {
        let output = "<error>file 99999 not found</error>\n";
        let filename = parse_filename_from_info_output(output);
        assert_eq!(filename, None);
    }
}
