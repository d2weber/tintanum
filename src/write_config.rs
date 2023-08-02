use crate::scheme_preference::SchemePreference;
use smol::fs;
use smol::fs::File;
use smol::io::AsyncBufReadExt;
use smol::io::AsyncWriteExt;
use smol::io::BufReader;
use smol::prelude::AsyncRead;
use smol::prelude::AsyncWrite;
use smol::stream::StreamExt;

pub(crate) async fn write_updated_config(
    rel_path: &str,
    p: SchemePreference,
) -> Result<(), std::io::Error> {
    let path = xdg::BaseDirectories::new()? // Could probably be cached
        .find_config_file(rel_path)
        .ok_or(std::io::Error::new(std::io::ErrorKind::NotFound, rel_path))?;

    let tmp_path = path.with_extension("auto_dark_theme.tmp");
    let mut out = File::create(&tmp_path).await?;
    if let e @ Err(_) = adjust_config(File::open(&path).await?, &mut out, p).await {
        fs::remove_file(tmp_path).await?;
        return e;
    };
    fs::rename(tmp_path, path).await?;
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
