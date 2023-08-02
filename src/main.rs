use crate::scheme_preference::SchemePreference;
use crate::settings_proxy::SettingChanged;
use crate::settings_proxy::SettingsProxy;
use smol::fs;
use smol::fs::File;
use smol::future;
use smol::io::AsyncBufReadExt;
use smol::io::AsyncWriteExt;
use smol::io::BufReader;
use smol::prelude::AsyncRead;
use smol::prelude::AsyncWrite;
use smol::process::Command;
use smol::stream::StreamExt;
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
    let (r1, r2) = future::zip(set_theme_alacritty(p), set_theme_helix(p)).await;
    r1?;
    r2
}

async fn set_theme_helix(p: SchemePreference) -> std::io::Result<()> {
    let path = {
        let mut path = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
        path.push(".config/helix/config.toml");
        path
    };

    let tmp_path = path.with_extension("tmp.toml");
    let mut out = File::create(&tmp_path).await?;

    if let e @ Err(_) = adjust_config(File::open(&path).await?, &mut out, p).await {
        out.close().await?;
        return e;
    };

    out.flush().await?;

    fs::rename(tmp_path, path).await?;
    Command::new("pkill").args(["-USR1", "hx"]).status().await?;

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
