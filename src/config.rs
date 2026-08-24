use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

/// config.toml の構造体定義
#[derive(Debug, Deserialize, Default, PartialEq)]
pub struct Config {
    pub host: Option<String>,
    pub ssh_user: Option<String>,
    pub identity_file: Option<PathBuf>,
    pub occ_path: Option<String>,
    pub php_path: Option<String>,
}

impl Config {
    /// 設定ファイルから Config 構造体を読み込む
    pub fn load(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        if path.exists() {
            let content = fs::read_to_string(path)?;
            Ok(parse_config_toml(&content)?)
        } else {
            Ok(Config::default())
        }
    }
}

/// TOML 文字列を Config 構造体にパースするヘルパー関数
pub fn parse_config_toml(content: &str) -> Result<Config, toml::de::Error> {
    toml::from_str(content)
}

// -----------------------------------------------------------------------------
// TOML設定パースのユニットテスト
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

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
}