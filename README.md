# nc-cli

SSH経由でリモートの Nextcloud サーバーに対して `occ` コマンドを実行・管理、およびローカルディレクトリの自動同期を行う CLI ツールです。

---

## 📑 目次

- [特徴](#特徴)
- [前提条件](#前提条件)
- [セットアップ](#セットアップ)
- [設定ファイル (config.toml)](#設定ファイル-configtoml)
- [コマンド一覧・使用例](#コマンド一覧使用例)
  - [ユーザー管理](#ユーザー管理)
  - [グループ管理](#グループ管理)
  - [ファイル操作・スキャン](#ファイル操作スキャン)
  - [ファイル同期とスキャン (`sync`)](#ファイル同期とスキャン-sync)
  - [システム・保守](#システム保守)
- [グローバルオプション](#グローバルオプション)
- [ライセンス](#ライセンス)

---

## 🚀 特徴

- **SSH接続による直接実行**: リモートサーバーへ SSH 接続し、指定された `php` および `occ` パスを用いて安全にコマンドを実行します。
- **設定ファイルと引数の柔軟な上書き**: `config.toml` によるデフォルト設定と、CLI 引数による一時的なパラメータ上書きに対応。
- **`rsync` による効率的な差分同期**: ローカルの `rsync_target` ディレクトリ内のファイルを、高速かつ安全にリモート側へ同期します。
- **自動キャッシュ更新 (files:scan)**: ファイル同期完了後、自動的に Nextcloud の `files:scan` を実行してデータベースへ反映。
- **JSON出力サポート**: `status`, `user-list`, `group-list`, `file-scan` などの主要コマンドで JSON 出力に対応。

---

## 🛠 前提条件

- [Rust](https://www.rust-lang.org/) (2021 edition 以降 / Cargo)
- `rsync` コマンド（ローカル環境にインストールされていること）
- SSH 接続可能な Nextcloud サーバー環境
- リモートサーバー側の SSH 秘密鍵認証（パスワードなし鍵、または ssh-agent）

---

## 📦 セットアップ

1. **リポジトリのクローン / 移動**
   ```bash
   git clone <repository-url>
   cd nc-cli

```

2. **設定ファイルの準備**
`config.example.toml` をコピーして `config.toml` を作成し、接続先サーバーの情報を入力します。
```bash
cp config.example.toml config.toml

```


3. **同期元フォルダの作成**
`src` と同じ階層に `rsync_target` フォルダを作成し、同期したいファイルを配置します。
```bash
mkdir rsync_target

```


4. **ビルド**
```bash
# デバッグビルド
cargo build

# リリースビルド
cargo build --release

```



---

## ⚙️ 設定ファイル (config.toml)

```toml
# Nextcloud サーバーの SSH 接続先と occ 実行パスを設定します
host = "example.server.jp:22"
ssh_user = "ssh_username"
identity_file = "/path/to/your/private/key"
php_path = "/usr/bin/php"
occ_path = "/var/www/nextcloud/occ"

```

※ CLI 実行時に `--host` や `--ssh-user` などのオプションを指定した場合、`config.toml` の値よりも CLI 引数が優先されます。

---

## 📖 コマンド一覧・使用例

### ユーザー管理

#### 1. ユーザーの作成 (`add`)

`occ user:add` を実行して新しいユーザーを作成します。

```bash
# パスワード自動生成で作成
cargo run -- add okamura --generate-password

# 表示名・グループ・メールアドレスを指定して作成
cargo run -- add okamura --generate-password --display-name="岡村" --group="admin" --email="okamura@example.com"

```

#### 2. 容量制限 (Quota) の設定 (`set-quota`)

`occ user:setting <uid> files quota <quota>` を実行します。

```bash
# 10 GB に設定
cargo run -- set-quota okamura "10 GB"

# 無制限に設定
cargo run -- set-quota okamura "unlimited"

```

#### 3. ユーザーの削除 (`user-delete`)

`occ user:delete` を実行します（対話確認をスキップして削除）。

```bash
cargo run -- user-delete -t okamura

```

#### 4. ユーザー一覧の取得 (`user-list`)

`occ user:list --info --output=json` を実行し、JSON形式でユーザー一覧を取得します。

```bash
cargo run -- user-list

```

---

### グループ管理

#### 1. グループ一覧の取得 (`group-list`)

`occ group:list --info --output=json` を実行し、JSON形式でグループ一覧を取得します。

```bash
cargo run -- group-list

```

---

### ファイル操作・スキャン

#### 1. ファイルの配置 (`file-put` / `files-put`)

サーバー内のローカルファイルを Nextcloud 上の指定パスまたはファイルIDに配置します (`occ files:put`)。

```bash
# サーバー内の一時ファイルを特定ユーザーのフォルダーへ配置
cargo run -- file-put /tmp/data.csv admin/files/Documents/data.csv

# 既存のファイルIDを指定して上書き
cargo run -- file-put /tmp/data.csv 12345

```

#### 2. ファイル/ディレクトリの削除 (`file-delete`)

ファイルIDまたはパスを指定して Nextcloud 上のファイルを削除します (`occ files:delete`)。

```bash
# パス指定で削除
cargo run -- file-delete "admin/files/Documents/old_file.txt"

# ファイルID指定で削除
cargo run -- file-delete 12345

```

#### 3. ファイルのスキャン・再同期 (`file-scan` / `files-scan`)

Nextcloud のファイルキャッシュと実ストレージを再同期します (`occ files:scan --output=json`)。

```bash
# 全ユーザーのファイルをスキャン
cargo run -- file-scan --all

# 特定ユーザーのファイルのみスキャン
cargo run -- file-scan admin

# 特定パスのみスキャン
cargo run -- file-scan -p "admin/files/Documents"

```

---

### ファイル同期とスキャン (`sync`)

ローカルの `rsync_target` ディレクトリからリモートディレクトリへ `rsync` で高速同期し、同期完了後に Nextcloud の `files:scan` を自動実行します。

※ `rsync_target` フォルダが存在しない、または空の場合は安全のためエラー終了します。

```bash
# 基本同期（指定パスへの転送）
cargo run -- sync /var/www/nextcloud/data/admin/files/sync_folder

# ドライラン (テスト実行：実際には転送・スキャンを行いません)
cargo run -- sync /var/www/nextcloud/data/admin/files/sync_folder -n

# 転送元に存在しないファイルをリモート側から削除 (--delete) + adminユーザーのキャシュを更新
cargo run -- sync /var/www/nextcloud/data/admin/files/sync_folder --delete --scan-user admin

# 任意ファイルの除外指定 + 特定パスのみスキャン
cargo run -- sync /var/www/nextcloud/data/admin/files/sync_folder -e "*.tmp" -e "build/" --scan-path "admin/files/sync_folder"

```

| オプション | 短縮 | デフォルト値 | 説明 |
| --- | --- | --- | --- |
| `-n, --dry-run` | `-n` | - | 試行運転を行います（ファイル転送・Nextcloud スキャンは実行されません）。 |
| `--delete` | - | - | 転送元（ローカル）に存在しないファイルを転送先（リモート）から削除します。 |
| `-e, --exclude <PATTERN>` | `-e` | `.DS_Store` | 同期対象から除外するパターンを指定します（複数指定可能）。 |
| `--scan-user <USER_ID>` | - | - | 同期完了後、指定したユーザーのファイルのみスキャンします。 |
| `--scan-path <PATH>` | - | - | 同期完了後、指定した相対パスのみスキャンします。 |
| `--scan-all` | - | - | 同期完了後、全ユーザーのファイルをスキャンします。 |

---

### システム・保守

#### 1. メンテナンスモードの切替 (`maintenance-mode`)

`occ maintenance:mode` を実行します。

```bash
# 有効化
cargo run -- maintenance-mode --on

# 無効化
cargo run -- maintenance-mode --off

# 現在のメンテナンスモード状態を確認
cargo run -- maintenance-mode

```

#### 2. ステータス情報の表示 (`status`)

`occ status --output=json` を実行し、Nextcloud 本体のバージョンや状態を JSON で取得します。

```bash
cargo run -- status

```

#### 3. OCC コマンド一覧の表示 (`list`)

利用可能な OCC コマンドの一覧を JSON 形式で表示します (`occ list --output=json`)。

```bash
cargo run -- list

```

---

## 🌐 グローバルオプション

すべてのサブコマンド共通で利用可能なフラグ・オプションです。

| オプション | 短縮 | デフォルト値 | 説明 |
| --- | --- | --- | --- |
| `-c, --config-file <FILE>` | `-c` | `config.toml` | 設定ファイルのパス |
| `--host <HOST>` | - | (config.toml) | SSH サーバーのホスト名とポート (例: `example.com:22`) |
| `-s, --ssh-user <USER>` | `-s` | (config.toml) | SSH ログインユーザー名 |
| `-i, --identity-file <KEY>` | `-i` | (config.toml) | SSH 秘密鍵ファイルのパス |
| `--occ-path <PATH>` | - | (config.toml) | Nextcloud の `occ` コマンドパス |
| `--php-path <PATH>` | - | (config.toml) | PHP 実行バイナリのパス |
| `-h, --help` | `-h` | - | ヘルプメッセージを表示 |
| `-V, --version` | `-V` | - | バージョン情報を表示 |

---

## 📄 ライセンス

本プロジェクトは [MIT License](https://www.google.com/search?q=LICENSE-MIT) および [Apache License 2.0](https://www.google.com/search?q=LICENSE-APACHE) のデュアルライセンスです。

```
