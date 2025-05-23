use image::{ImageBuffer, Luma};
use poise::serenity_prelude::CreateAttachment;
use rusttype::{Font, Rect, Scale, point};
use std::io::Cursor;

use crate::{Context, Error};

#[poise::command(slash_command)]
pub async fn send_glyph_image(ctx: Context<'_>) -> Result<(), Error> {
    // Load font
    let font_data = include_bytes!("C:\\Users\\yoish\\git\\rust\\jplearnbot\\fonts\\font.ttf");
    let font = Font::try_from_bytes(font_data as &[u8]).unwrap();

    let scale = Scale::uniform(40.0);
    let text = "Hello, world!";
    let start = point(0.0, 40.0);

    // Layout glyphs for the whole string
    let glyphs: Vec<_> = font.layout(text, scale, start).collect();

    // Calculate bounding box for all glyphs
    let bb = glyphs
        .iter()
        .filter_map(|g| g.pixel_bounding_box())
        .fold(None::<rusttype::Rect<i32>>, |acc, r| {
            Some(match acc {
                Some(acc) => rusttype::Rect {
                    min: rusttype::point(acc.min.x.min(r.min.x), acc.min.y.min(r.min.y)),
                    max: rusttype::point(acc.max.x.max(r.max.x), acc.max.y.max(r.max.y)),
                },
                None => r,
            })
        })
        .unwrap();

    let padding = 10;
    let padded_bb = rusttype::Rect {
        min: rusttype::point(bb.min.x - padding, bb.min.y - padding),
        max: rusttype::point(bb.max.x + padding, bb.max.y + padding),
    };

    let width = padded_bb.width() as u32;
    let height = padded_bb.height() as u32;

    // White background
    let mut image = ImageBuffer::<Luma<u8>, Vec<u8>>::from_pixel(width, height, Luma([255u8]));

    // Draw each glyph as black on white background
    for glyph in glyphs {
        if let Some(g_bb) = glyph.pixel_bounding_box() {
            glyph.draw(|x, y, v| {
                let px = (x as i32 + g_bb.min.x - padded_bb.min.x) as u32;
                let py = (y as i32 + g_bb.min.y - padded_bb.min.y) as u32;
                if px < width && py < height {
                    let alpha = (v * 255.0) as u8;
                    let pixel_val = 255u8.saturating_sub(alpha);
                    image.put_pixel(px, py, Luma([pixel_val]));
                }
            });
        }
    }

    // Encode as PNG into memory
    let mut buf = Cursor::new(Vec::new());
    image::DynamicImage::ImageLuma8(image)
        .write_to(&mut buf, image::ImageFormat::Png)
        .unwrap();

    // Send image as attachment
    ctx.send(
        poise::CreateReply::default()
            .attachment(CreateAttachment::bytes(buf.into_inner(), "text.png")),
    )
    .await?;

    Ok(())
}
