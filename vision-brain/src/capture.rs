use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD, Engine};

/// Capture the primary screen and return base64-encoded PNG.
/// Runs in spawn_blocking because screen capture may take 50–200 ms.
pub async fn capture_primary() -> Result<String> {
    tokio::task::spawn_blocking(|| -> Result<String> {
        let screens = screenshots::Screen::all().map_err(|e| anyhow!("screens: {e:?}"))?;
        let screen = screens.first().ok_or_else(|| anyhow!("no screen detected"))?;
        let img = screen.capture().map_err(|e| anyhow!("capture: {e:?}"))?;
        // screenshots bundles its own image crate; convert via raw RGBA bytes
        let (width, height) = (img.width(), img.height());
        let rgba_bytes = img.into_raw();
        let dyn_img = image::DynamicImage::ImageRgba8(
            image::RgbaImage::from_raw(width, height, rgba_bytes)
                .ok_or_else(|| anyhow!("failed to build RgbaImage"))?,
        );
        let png = encode_to_png(&dyn_img)?;
        Ok(STANDARD.encode(&png))
    })
    .await?
}

fn encode_to_png(img: &image::DynamicImage) -> Result<Vec<u8>> {
    let mut png_data = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut png_data),
        image::ImageFormat::Png,
    )
    .map_err(|e| anyhow!("PNG encode: {e}"))?;
    Ok(png_data)
}
