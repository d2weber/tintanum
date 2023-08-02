use crate::color_scheme::SchemePreference;
use crate::color_scheme::SchemeProxy;
use crate::write_config::write_updated_config;
use smol::future;
use smol::prelude::*;
use smol::process::Command;

mod color_scheme;
mod write_config;

fn main() -> zbus::Result<()> {
    smol::block_on(async {
        let scheme = SchemeProxy::new().await?;
        let mut stream = scheme.receive_changed().await?;
        while let Some(preference) = stream.next().await {
            set_theme(preference).await;
        }
        Ok(())
    })
}

async fn set_theme(p: SchemePreference) {
    println!("Setting preference {:?}", p);
    let (r1, r2) = future::zip(set_theme_alacritty(p), set_theme_helix(p)).await;
    if let Err(e) = r1.and(r2) {
        eprintln!("Error: {e}");
    };
}

async fn set_theme_alacritty(p: SchemePreference) -> std::io::Result<()> {
    write_updated_config("alacritty/alacritty.yml", p).await
}

async fn set_theme_helix(p: SchemePreference) -> std::io::Result<()> {
    write_updated_config("helix/config.toml", p).await?;
    Command::new("pkill").args(["-USR1", "hx"]).status().await?;
    Ok(())
}
