use crate::settings_proxy::SchemePreference;
use crate::settings_proxy::SchemeProxy;
use crate::write_config::write_updated_config;
use smol::future;
use smol::process::Command;
use smol::stream::StreamExt;

mod settings_proxy;
mod write_config;

fn main() -> zbus::Result<()> {
    smol::block_on(async {
        let proxy = SchemeProxy::new().await?;

        let Ok(mut preference) = proxy.read().await else {
            panic! {"Couldn't read color scheme preference. Are you using D-Bus?"}
        };
        println!("Read inital preference {:?}", preference);
        if let Err(e) = set_theme(preference).await {
            eprintln!("{e:?}")
        };

        let mut stream = proxy.receive_scheme_changed().await?;
        while let Some(new_preference) = stream.next().await {
            if preference == new_preference {
                continue;
            }
            preference = new_preference;
            println!("Got preference {:?}", new_preference);
            if let Err(e) = set_theme(new_preference).await {
                eprintln!("{e:?}")
            };
        }
        Ok(())
    })
}

async fn set_theme(p: SchemePreference) -> std::io::Result<()> {
    let (r1, r2) = future::zip(set_theme_alacritty(p), set_theme_helix(p)).await;
    r1?;
    r2
}

async fn set_theme_alacritty(p: SchemePreference) -> std::io::Result<()> {
    write_updated_config("alacritty/alacritty.yml", p).await
}

async fn set_theme_helix(p: SchemePreference) -> std::io::Result<()> {
    write_updated_config("helix/config.toml", p).await?;
    Command::new("pkill").args(["-USR1", "hx"]).status().await?;
    Ok(())
}
