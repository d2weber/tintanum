use async_fs::File;
use futures_lite::io::BufReader;
use smol::future;
use smol::prelude::*;
use smol::process::Command;
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

pub async fn write_updated_config(
    path: impl AsRef<Path>,
    p: SchemePreference,
) -> Result<(), std::io::Error> {
    let tmp_path = path.as_ref().with_extension("auto_dark_theme.tmp");
    let mut out = File::create(&tmp_path).await?;
    if let e @ Err(_) = adjust_config(File::open(&path).await?, &mut out, p).await {
        async_fs::remove_file(tmp_path).await?;
        return e;
    };
    async_fs::rename(tmp_path, path).await?;
    Ok(())
}

async fn adjust_config(
    inp: impl AsyncRead + Unpin,
    mut out: impl AsyncWrite + Unpin,
    p: SchemePreference,
) -> Result<(), std::io::Error> {
    let [mut old_tag, mut new_tag] = match p {
        SchemePreference::Dark => ["#[light]", "#[dark]"],
        _ => ["#[dark]", "#[light]"],
    }
    .map(Some);

    let mut lines = BufReader::new(inp).lines();
    while let Some(Ok(line)) = lines.next().await {
        let trimmed = line.trim();
        if new_tag.is_some_and(|t| trimmed.ends_with(t)) {
            new_tag.take().unwrap(); // Mark as found
            if trimmed.starts_with('#') {
                let (trailing_whitespaces, rest) = line.split_once('#').unwrap();
                out.write_all(trailing_whitespaces.as_bytes()).await?;
                out.write_all(rest.strip_prefix(' ').unwrap_or(rest).as_bytes())
                    .await?;
            } else {
                out.write_all(line.as_bytes()).await?;
            }
        } else {
            if old_tag.is_some_and(|t| trimmed.ends_with(t)) {
                old_tag.take().unwrap(); // Mark as found
                if !trimmed.starts_with('#') {
                    out.write_all(b"# ").await?;
                }
            }
            out.write_all(line.as_bytes()).await?;
        }
        out.write_all(b"\n").await?;
    }
    if let Some(t) = old_tag.or(new_tag) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Tag `{t}` not found"),
        ));
    }
    out.flush().await?;

    Ok(())
}
