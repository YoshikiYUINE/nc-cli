use clap::Parser;
use std::process;

// ※ Cargo.toml の [package] name に指定されているパッケージ名（ハイフンはアンダースコア化）
// 例: name = "nc-cli" の場合は `nc_cli` と指定します。
use nc_cli::cli::Args;

fn main() {
    let args = Args::parse();

    if let Err(e) = nc_cli::run(args) {
        eprintln!("{}", e);
        process::exit(1);
    }
}