use smol::future;
use smol::prelude::*;
use smol::process::Command;
use tintanum::write_config::write_updated_config;
use tintanum::SchemePreference;
use tintanum::SchemeProxy;
use xdg::BaseDirectories;

use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

fn main() -> zbus::Result<()> {
    smol::block_on(async {
        let scheme = SchemeProxy::new(&zbus::Connection::session().await?).await?;
        let mut stream = scheme.init_and_receive_changed().await?;
        while let Some(preference) = stream.next().await {
            set_theme(preference).await;
        }
        Ok(())
    })
}

async fn set_theme(p: SchemePreference) {
    println!("Setting preference {p:?}");
    let (r1, r2) = future::zip(set_theme_alacritty(p), set_theme_helix(p)).await;
    if let Err(e) = r1.and(r2) {
        eprintln!("Error: {e}");
    };
}

async fn set_theme_alacritty(p: SchemePreference) -> std::io::Result<()> {
    write_updated_config(find_config("alacritty/alacritty.yml")?, p).await
}

async fn set_theme_helix(p: SchemePreference) -> std::io::Result<()> {
    write_updated_config(find_config("helix/config.toml")?, p).await?;
    Command::new("pkill").args(["-USR1", "hx"]).status().await?;
    Ok(())
}

static BASE_DIRS: OnceLock<Result<BaseDirectories, xdg::BaseDirectoriesError>> = OnceLock::new();

fn find_config(rel_path: impl AsRef<Path>) -> std::io::Result<PathBuf> {
    BASE_DIRS
        .get_or_init(|| BaseDirectories::new())
        .as_ref()
        .map_err(|_| std::io::ErrorKind::NotFound)?
        .find_config_file(rel_path)
        .ok_or(std::io::ErrorKind::NotFound.into())
}
