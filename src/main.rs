use crate::scheme_preference::SchemePreference;
use crate::settings_proxy::SettingChanged;
use crate::settings_proxy::SettingsProxy;
use smol::fs;
use smol::fs::read_to_string;
use smol::fs::File;
use smol::io::AsyncWriteExt;
use smol::process::Command;
use smol::stream::StreamExt;
use toml_edit::{value, Document};
use zbus::Connection;

mod scheme_preference;
mod settings_proxy;

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
    let t1 = smol::spawn(set_theme_alacritty(p));
    let t2 = smol::spawn(set_theme_helix(p));
    t1.await?;
    t2.await
}

async fn set_theme_helix(p: SchemePreference) -> std::io::Result<()> {
    let path = {
        let mut path = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
        path.push(".config/helix/config.toml");
        path
    };

    let mut config = read_to_string(&path)
        .await?
        .parse::<Document>()
        .expect("Couldn't parse config file");

    let theme = match p {
        SchemePreference::Dark => "adwaita-dark",
        _ => "onelight",
    };
    config["theme"] = value(theme);

    let tmp_path = path.with_extension("tmp.toml");

    let mut file = File::create(&tmp_path).await?;
    file.write(config.to_string().as_bytes()).await?;
    file.flush().await?;

    fs::rename(tmp_path, path).await?;
    Command::new("pkill").args(["-USR1", "hx"]).status().await?;

    Ok(())
}

async fn set_theme_alacritty(p: SchemePreference) -> std::io::Result<()> {
    let dst = {
        let mut path = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
        path.push(".config/alacritty/current_auto_theme.yml");
        path
    };
    let theme = match p {
        SchemePreference::Dark => "theme-dark.yml",
        _ => "theme-light.yml",
    };

    if dst.exists() {
        fs::remove_file(&dst).await?;
    }

    fs::hard_link(dst.with_file_name(theme), dst).await
}

const NAMESPACE: &'static str = "org.freedesktop.appearance";
const KEY: &'static str = "color-scheme";

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
