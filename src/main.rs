use crate::scheme_preference::SchemePreference;
use crate::settings_proxy::SettingChanged;
use crate::settings_proxy::SettingsProxy;
use crate::write_config::write_updated_config;
use smol::future;
use smol::process::Command;
use smol::stream::StreamExt;
use zbus::Connection;

mod scheme_preference;
mod settings_proxy;
mod write_config;

fn main() -> zbus::Result<()> {
    smol::block_on(async {
        let conn = Connection::session().await?;
        let proxy = SettingsProxy::new(&conn).await?;

        let Ok(mut preference) = read_scheme_preference(&proxy).await else {
            panic! {"Couldn't read color scheme preference. Are you using D-Bus?"}
        };
        println!("Read inital preference {:?}", preference);
        if let Err(e) = set_theme(preference).await {
            eprintln!("{e:?}")
        };

        let mut stream = proxy.receive_setting_changed().await?;
        while let Some(signal) = stream.next().await {
            if let Ok(new_preference) = extract_scheme_preference(signal) {
                if preference == new_preference {
                    continue;
                }
                preference = new_preference;
                println!("Got preference {:?}", new_preference);
                if let Err(e) = set_theme(new_preference).await {
                    eprintln!("{e:?}")
                };
            }
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

const NAMESPACE: &str = "org.freedesktop.appearance";
const KEY: &str = "color-scheme";

async fn read_scheme_preference(proxy: &SettingsProxy<'_>) -> zbus::Result<SchemePreference> {
    let v = proxy.read(NAMESPACE, KEY).await?;
    let v = v.downcast_ref::<zbus::zvariant::Value>().unwrap().clone();
    SchemePreference::try_from(v)
}

fn extract_scheme_preference(signal: SettingChanged) -> zbus::Result<SchemePreference> {
    let args = signal.args()?;
    if !(args.namespace == NAMESPACE && args.key == KEY) {
        return Err(zbus::Error::MissingField);
    }
    SchemePreference::try_from(args.value)
}
