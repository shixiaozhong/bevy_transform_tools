use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use super::{
    ORIENTATION_LABEL_GLYPH_SCALE, ORIENTATION_LABEL_GLYPH_SPACING, ORIENTATION_LABEL_TEXTURE_SIZE,
};

pub(super) fn orientation_face_texture_paths(direction: Vec3) -> (&'static str, &'static str) {
    if direction.z > 0.0 {
        ("viewcube/top.png", "viewcube/top-hover.png")
    } else if direction.z < 0.0 {
        ("viewcube/bottom.png", "viewcube/bottom-hover.png")
    } else if direction.y > 0.0 {
        ("viewcube/back.png", "viewcube/back-hover.png")
    } else if direction.y < 0.0 {
        ("viewcube/front.png", "viewcube/front-hover.png")
    } else if direction.x < 0.0 {
        ("viewcube/left.png", "viewcube/left-hover.png")
    } else {
        ("viewcube/right.png", "viewcube/right-hover.png")
    }
}

pub(super) fn orientation_label_texture(
    label: &str,
    text_color: [u8; 4],
    background: [u8; 4],
) -> Image {
    let size = ORIENTATION_LABEL_TEXTURE_SIZE;
    let mut pixels = vec![0; (size * size * 4) as usize];

    for chunk in pixels.chunks_exact_mut(4) {
        chunk.copy_from_slice(&background);
    }

    let glyphs = label.chars().collect::<Vec<_>>();
    let glyph_width = 5 * ORIENTATION_LABEL_GLYPH_SCALE;
    let glyph_height = 7 * ORIENTATION_LABEL_GLYPH_SCALE;
    let text_width = glyph_width * glyphs.len() as u32
        + ORIENTATION_LABEL_GLYPH_SPACING * glyphs.len().saturating_sub(1) as u32;
    let mut x = (size.saturating_sub(text_width)) / 2;
    let y = (size.saturating_sub(glyph_height)) / 2;

    for glyph in glyphs {
        draw_orientation_glyph(&mut pixels, size, glyph, x, y, text_color);
        x += glyph_width + ORIENTATION_LABEL_GLYPH_SPACING;
    }

    Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn draw_orientation_glyph(
    pixels: &mut [u8],
    texture_size: u32,
    glyph: char,
    x: u32,
    y: u32,
    color: [u8; 4],
) {
    for (row, pattern) in orientation_glyph_pattern(glyph).iter().enumerate() {
        for (column, value) in pattern.as_bytes().iter().enumerate() {
            if *value == b'#' {
                draw_orientation_label_cell(
                    pixels,
                    texture_size,
                    x + column as u32 * ORIENTATION_LABEL_GLYPH_SCALE,
                    y + row as u32 * ORIENTATION_LABEL_GLYPH_SCALE,
                    color,
                );
            }
        }
    }
}

fn draw_orientation_label_cell(
    pixels: &mut [u8],
    texture_size: u32,
    x: u32,
    y: u32,
    color: [u8; 4],
) {
    for py in y..(y + ORIENTATION_LABEL_GLYPH_SCALE) {
        for px in x..(x + ORIENTATION_LABEL_GLYPH_SCALE) {
            if px >= texture_size || py >= texture_size {
                continue;
            }
            let index = ((py * texture_size + px) * 4) as usize;
            pixels[index..index + 4].copy_from_slice(&color);
        }
    }
}

fn orientation_glyph_pattern(glyph: char) -> [&'static str; 7] {
    match glyph {
        '-' => [
            ".....", ".....", ".....", "#####", ".....", ".....", ".....",
        ],
        'X' | 'x' => [
            "#...#", ".#.#.", "..#..", "..#..", "..#..", ".#.#.", "#...#",
        ],
        'Y' | 'y' => [
            "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#..", "..#..",
        ],
        'Z' | 'z' => [
            "#####", "....#", "...#.", "..#..", ".#...", "#....", "#####",
        ],
        _ => [
            ".....", ".....", ".....", ".....", ".....", ".....", ".....",
        ],
    }
}
