use crate::scheme_preference::SchemePreference;
use crate::settings_proxy::SettingChanged;
use crate::settings_proxy::SettingsProxy;
use futures_util::stream::StreamExt;
use smol::fs;
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
    let dst = {
        let mut path = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
        path.push(".config/alacritty/current_auto_theme.yml");
        path
    };
    let theme = match p {
        SchemePreference::NoPreference | SchemePreference::Light => {
            dst.with_file_name("theme-light.yml")
        }
        SchemePreference::Dark => dst.with_file_name("theme-dark.yml"),
    };
    match fs::metadata(&dst).await {
        Ok(existing) => {
            if existing.is_file() {
                if let Err(e) = fs::remove_file(&dst).await {
                    return Err(e);
                };
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "Auto theme exists but it isn't a file",
                ));
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
        Err(e) => return Err(e),
    }
    fs::hard_link(theme, dst).await
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
