#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Kind {
    Mouse,
    Headset,
    Keyboard,
}

impl Kind {
    pub const ALL: [Self; 3] = [Self::Mouse, Self::Headset, Self::Keyboard];

    pub fn name(self) -> &'static str {
        match self {
            Self::Mouse => "Mouse",
            Self::Headset => "Headset",
            Self::Keyboard => "Keyboard",
        }
    }

    pub fn classify(text: &str) -> Option<Self> {
        if text.contains("mouse") {
            Some(Self::Mouse)
        } else if text.contains("headset") || text.contains("headphone") {
            Some(Self::Headset)
        } else if text.contains("keyboard") {
            Some(Self::Keyboard)
        } else {
            None
        }
    }
}

pub fn level_color(percentage: u8, critical: u8, low: u8, normal: [u8; 3]) -> [u8; 3] {
    if percentage <= critical {
        [255, 72, 72]
    } else if percentage <= low {
        [255, 185, 0]
    } else {
        normal
    }
}

fn rounded(x: f32, y: f32, left: f32, top: f32, right: f32, bottom: f32, radius: f32) -> bool {
    let dx = x - x.clamp(left + radius, right - radius);
    let dy = y - y.clamp(top + radius, bottom - radius);
    dx * dx + dy * dy <= radius * radius
}

fn glyph(kind: Kind, x: f32, y: f32) -> bool {
    match kind {
        Kind::Mouse => {
            rounded(x, y, 0.18, 0.0, 0.82, 1.0, 0.32)
                && !rounded(x, y, 0.43, 0.15, 0.57, 0.42, 0.07)
        }
        Kind::Headset => {
            let radius = ((x - 0.5).powi(2) + (y - 0.4).powi(2)).sqrt();
            (y <= 0.4 && (0.28..=0.4).contains(&radius))
                || ((0.4..=0.6).contains(&y)
                    && ((0.1..=0.22).contains(&x) || (0.78..=0.9).contains(&x)))
                || rounded(x, y, 0.02, 0.5, 0.3, 1.0, 0.1)
                || rounded(x, y, 0.7, 0.5, 0.98, 1.0, 0.1)
        }
        Kind::Keyboard => {
            rounded(x, y, 0.0, 0.18, 1.0, 0.82, 0.14)
                && !(0..3).any(|col| {
                    (0.2 + col as f32 * 0.235..=0.33 + col as f32 * 0.235).contains(&x)
                        && (0.31..=0.44).contains(&y)
                })
                && !rounded(x, y, 0.2, 0.56, 0.8, 0.69, 0.05)
        }
    }
}

pub fn tray_rgba(
    kind: Kind,
    percentage: Option<u8>,
    size: u32,
    dark: bool,
    critical: u8,
    low: u8,
) -> Vec<u8> {
    let foreground = if dark { [255, 255, 255] } else { [32, 32, 32] };
    let fill = level_color(percentage.unwrap_or(100), critical, low, foreground);
    let bar = (size as f32 / 8.0).round().max(2.0);
    let gap = (size as f32 / 16.0).round().max(1.0);
    let gs = size as f32 - bar - gap;
    let pad = gap;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let mut sums = [0u32; 4];
            for sy in 0..8 {
                for sx in 0..8 {
                    let px = x as f32 + (sx as f32 + 0.5) / 8.0;
                    let py = y as f32 + (sy as f32 + 0.5) / 8.0;
                    let mut color = [0; 4];
                    if glyph(kind, (px - (size as f32 - gs) / 2.0) / gs, py / gs) {
                        color = [
                            foreground[0],
                            foreground[1],
                            foreground[2],
                            if percentage.is_some() { 255 } else { 110 },
                        ];
                    }
                    if rounded(
                        px,
                        py,
                        pad,
                        size as f32 - bar,
                        size as f32 - pad,
                        size as f32,
                        bar / 2.0,
                    ) {
                        color = [foreground[0], foreground[1], foreground[2], 70];
                        if let Some(pct) = percentage.filter(|&p| p > 0) {
                            let width = ((size as f32 - 2.0 * pad) * pct as f32 / 100.0)
                                .round()
                                .max(bar);
                            if rounded(
                                px,
                                py,
                                pad,
                                size as f32 - bar,
                                pad + width,
                                size as f32,
                                bar / 2.0,
                            ) {
                                color = [fill[0], fill[1], fill[2], 255];
                            }
                        }
                    }
                    for c in 0..3 {
                        sums[c] += color[c] as u32 * color[3] as u32;
                    }
                    sums[3] += color[3] as u32;
                }
            }
            let alpha = sums[3];
            for sum in &sums[..3] {
                rgba.push(sum.checked_div(alpha).unwrap_or(0) as u8);
            }
            rgba.push((alpha / 64) as u8);
        }
    }
    rgba
}

pub fn app_rgba(size: u32) -> Vec<u8> {
    let mut image = vec![0; (size * size * 4) as usize];
    let inner = (size as f32 * 0.6).round() as u32;
    let mark = tray_rgba(Kind::Mouse, Some(70), inner, true, 15, 30);
    let offset = (size - inner) / 2;
    for y in 0..size {
        for x in 0..size {
            let p = ((y * size + x) * 4) as usize;
            if rounded(
                x as f32 + 0.5,
                y as f32 + 0.5,
                0.0,
                0.0,
                size as f32,
                size as f32,
                size as f32 * 0.24,
            ) {
                let light = (16.0 * (1.0 - y as f32 / size as f32)) as u8;
                image[p..p + 4].copy_from_slice(&[20 + light, 22 + light, 28 + light, 255]);
            }
            if x >= offset && y >= offset && x < offset + inner && y < offset + inner {
                let i = (((y - offset) * inner + x - offset) * 4) as usize;
                let alpha = mark[i + 3] as u32;
                let color = if y - offset >= inner - (inner / 8).max(2) {
                    [0, 190, 255]
                } else {
                    [255, 255, 255]
                };
                for c in 0..3 {
                    image[p + c] = ((color[c] as u32 * alpha + image[p + c] as u32 * (255 - alpha))
                        / 255) as u8;
                }
            }
        }
    }
    image
}

pub fn ico(frames: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut data = vec![0, 0, 1, 0];
    data.extend_from_slice(&(frames.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * frames.len() as u32;
    let mut images = Vec::new();
    for (size, rgba) in frames {
        let mask_stride = size.div_ceil(32) * 4;
        let bytes = 40 + size * size * 4 + mask_stride * size;
        data.extend_from_slice(&[*size as u8, *size as u8, 0, 0, 1, 0, 32, 0]);
        data.extend_from_slice(&bytes.to_le_bytes());
        data.extend_from_slice(&offset.to_le_bytes());
        offset += bytes;
        images.extend_from_slice(&40u32.to_le_bytes());
        images.extend_from_slice(&size.to_le_bytes());
        images.extend_from_slice(&(size * 2).to_le_bytes());
        images.extend_from_slice(&[1, 0, 32, 0]);
        images.extend_from_slice(&[0; 24]);
        for y in (0..*size).rev() {
            for x in 0..*size {
                let p = ((y * size + x) * 4) as usize;
                images.extend_from_slice(&[rgba[p + 2], rgba[p + 1], rgba[p], rgba[p + 3]]);
            }
        }
        images.resize(images.len() + (mask_stride * size) as usize, 0);
    }
    data.extend(images);
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ico_directory_points_to_complete_native_bitmap_frames() {
        let frames = [16, 32, 256].map(|size| (size, app_rgba(size)));
        let image = ico(&frames);
        assert_eq!(&image[..6], &[0, 0, 1, 0, 3, 0]);
        for (index, (size, _)) in frames.iter().enumerate() {
            let entry = 6 + index * 16;
            let offset =
                u32::from_le_bytes(image[entry + 12..entry + 16].try_into().unwrap()) as usize;
            let bytes =
                u32::from_le_bytes(image[entry + 8..entry + 12].try_into().unwrap()) as usize;
            assert!(offset + bytes <= image.len());
            assert_eq!(
                u32::from_le_bytes(image[offset..offset + 4].try_into().unwrap()),
                40
            );
            assert_eq!(
                u32::from_le_bytes(image[offset + 4..offset + 8].try_into().unwrap()),
                *size
            );
            assert_eq!(
                u32::from_le_bytes(image[offset + 8..offset + 12].try_into().unwrap()),
                size * 2
            );
        }
    }
}
