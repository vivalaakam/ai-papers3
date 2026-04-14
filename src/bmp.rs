use alloc::vec;
use alloc::vec::Vec;
use miniz_oxide::inflate::decompress_to_vec_zlib_with_limit;

pub struct BmpImage {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16, &'static str> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or("BMP truncated while reading u16")?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32, &'static str> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or("BMP truncated while reading u32")?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_be_u32(data: &[u8], offset: usize) -> Result<u32, &'static str> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or("PNG truncated while reading u32")?;
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_i32(data: &[u8], offset: usize) -> Result<i32, &'static str> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or("BMP truncated while reading i32")?;
    Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn grayscale_from_rgb(r: u8, g: u8, b: u8) -> u8 {
    let gray = (u32::from(r) * 30 + u32::from(g) * 59 + u32::from(b) * 11) / 100;
    ((gray as u8) & 0xF0).max(0x10)
}

fn grayscale_with_alpha(gray: u8, alpha: u8) -> u8 {
    let gray = (u32::from(gray) * u32::from(alpha) + 255 * u32::from(255 - alpha)) / 255;
    ((gray as u8) & 0xF0).max(0x10)
}

pub fn decode_bmp(data: &[u8]) -> Result<BmpImage, &'static str> {
    if data.len() < 54 || &data[0..2] != b"BM" {
        return Err("Unsupported BMP header");
    }

    let pixel_offset = read_u32(data, 10)? as usize;
    let dib_size = read_u32(data, 14)?;
    if dib_size < 40 {
        return Err("Unsupported BMP DIB header");
    }

    let width = read_i32(data, 18)?;
    let height_signed = read_i32(data, 22)?;
    let planes = read_u16(data, 26)?;
    let bpp = read_u16(data, 28)?;
    let compression = read_u32(data, 30)?;

    if planes != 1 {
        return Err("Unsupported BMP planes");
    }
    if compression != 0 {
        return Err("Compressed BMP is not supported");
    }
    if width <= 0 || height_signed == 0 {
        return Err("Invalid BMP dimensions");
    }

    let width = width as usize;
    let top_down = height_signed < 0;
    let height = height_signed.unsigned_abs() as usize;
    let bytes_per_pixel = match bpp {
        24 => 3usize,
        32 => 4usize,
        _ => return Err("Only 24-bit and 32-bit BMP are supported"),
    };
    let row_stride = (width * bytes_per_pixel).div_ceil(4) * 4;

    let mut pixels = vec![0xF0; width * height];
    for row in 0..height {
        let src_row = if top_down { row } else { height - 1 - row };
        let row_start = pixel_offset + src_row * row_stride;
        let row_end = row_start + width * bytes_per_pixel;
        let row_data = data
            .get(row_start..row_end)
            .ok_or("BMP pixel data is truncated")?;

        for col in 0..width {
            let pixel_start = col * bytes_per_pixel;
            let b = row_data[pixel_start];
            let g = row_data[pixel_start + 1];
            let r = row_data[pixel_start + 2];
            pixels[row * width + col] = grayscale_from_rgb(r, g, b);
        }
    }

    Ok(BmpImage {
        width,
        height,
        pixels,
    })
}

fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let a = i32::from(a);
    let b = i32::from(b);
    let c = i32::from(c);
    let p = a + b - c;
    let pa = (p - a).abs();
    let pb = (p - b).abs();
    let pc = (p - c).abs();

    if pa <= pb && pa <= pc {
        a as u8
    } else if pb <= pc {
        b as u8
    } else {
        c as u8
    }
}

pub fn decode_png(data: &[u8]) -> Result<BmpImage, &'static str> {
    const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

    if data.len() < PNG_SIGNATURE.len() || &data[..8] != PNG_SIGNATURE {
        return Err("Unsupported PNG header");
    }

    let mut cursor = 8usize;
    let mut width = 0usize;
    let mut height = 0usize;
    let mut color_type = 0u8;
    let mut seen_ihdr = false;
    let mut idat = Vec::new();

    while cursor < data.len() {
        let length = read_be_u32(data, cursor)? as usize;
        let chunk_type = data
            .get(cursor + 4..cursor + 8)
            .ok_or("PNG truncated while reading chunk type")?;
        let chunk_data = data
            .get(cursor + 8..cursor + 8 + length)
            .ok_or("PNG truncated while reading chunk data")?;
        let next_cursor = cursor + 12 + length;
        data.get(cursor + 8 + length..next_cursor)
            .ok_or("PNG truncated while reading chunk CRC")?;

        match chunk_type {
            b"IHDR" => {
                if length != 13 {
                    return Err("Invalid PNG IHDR length");
                }

                width = read_be_u32(chunk_data, 0)? as usize;
                height = read_be_u32(chunk_data, 4)? as usize;
                let bit_depth = chunk_data[8];
                color_type = chunk_data[9];
                let compression = chunk_data[10];
                let filter = chunk_data[11];
                let interlace = chunk_data[12];

                if width == 0 || height == 0 {
                    return Err("Invalid PNG dimensions");
                }
                if bit_depth != 8 {
                    return Err("Only 8-bit PNG is supported");
                }
                if compression != 0 || filter != 0 {
                    return Err("Unsupported PNG compression or filter method");
                }
                if interlace != 0 {
                    return Err("Interlaced PNG is not supported");
                }

                seen_ihdr = true;
            }
            b"IDAT" => idat.extend_from_slice(chunk_data),
            b"IEND" => break,
            _ => {}
        }

        cursor = next_cursor;
    }

    if !seen_ihdr {
        return Err("PNG is missing IHDR");
    }
    if idat.is_empty() {
        return Err("PNG is missing IDAT");
    }

    let channels = match color_type {
        0 => 1usize,
        2 => 3usize,
        4 => 2usize,
        6 => 4usize,
        _ => return Err("Unsupported PNG color type"),
    };
    let bytes_per_pixel = channels;
    let stride = width
        .checked_mul(bytes_per_pixel)
        .ok_or("PNG row stride overflow")?;
    let expected_size = height
        .checked_mul(stride + 1)
        .ok_or("PNG buffer size overflow")?;
    let decompressed = decompress_to_vec_zlib_with_limit(&idat, expected_size)
        .map_err(|_| "PNG inflate failed")?;

    if decompressed.len() != expected_size {
        return Err("PNG decoded size mismatch");
    }

    let mut pixels = vec![0xF0; width * height];
    let mut prev_row = vec![0u8; stride];
    let mut row = vec![0u8; stride];

    for y in 0..height {
        let filter = decompressed[y * (stride + 1)];
        let scanline = &decompressed[y * (stride + 1) + 1..(y + 1) * (stride + 1)];

        for x in 0..stride {
            let raw = scanline[x];
            let left = if x >= bytes_per_pixel {
                row[x - bytes_per_pixel]
            } else {
                0
            };
            let up = prev_row[x];
            let up_left = if x >= bytes_per_pixel {
                prev_row[x - bytes_per_pixel]
            } else {
                0
            };

            row[x] = match filter {
                0 => raw,
                1 => raw.wrapping_add(left),
                2 => raw.wrapping_add(up),
                3 => raw.wrapping_add(((u16::from(left) + u16::from(up)) / 2) as u8),
                4 => raw.wrapping_add(paeth_predictor(left, up, up_left)),
                _ => return Err("Unsupported PNG row filter"),
            };
        }

        for x in 0..width {
            let grayscale = match color_type {
                0 => grayscale_with_alpha(row[x], 255),
                2 => {
                    let base = x * 3;
                    grayscale_from_rgb(row[base], row[base + 1], row[base + 2])
                }
                4 => {
                    let base = x * 2;
                    grayscale_with_alpha(row[base], row[base + 1])
                }
                6 => {
                    let base = x * 4;
                    let alpha = row[base + 3];
                    let r = ((u32::from(row[base]) * u32::from(alpha)
                        + 255 * u32::from(255 - alpha))
                        / 255) as u8;
                    let g = ((u32::from(row[base + 1]) * u32::from(alpha)
                        + 255 * u32::from(255 - alpha))
                        / 255) as u8;
                    let b = ((u32::from(row[base + 2]) * u32::from(alpha)
                        + 255 * u32::from(255 - alpha))
                        / 255) as u8;
                    grayscale_from_rgb(r, g, b)
                }
                _ => unreachable!(),
            };
            pixels[y * width + x] = grayscale;
        }

        prev_row.copy_from_slice(&row);
    }

    Ok(BmpImage {
        width,
        height,
        pixels,
    })
}
