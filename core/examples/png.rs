//! Codificador PNG mínimo para las herramientas de examples/.

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

fn chunk(out: &mut Vec<u8>, kind: &[u8], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut body = kind.to_vec();
    body.extend_from_slice(data);
    out.extend_from_slice(&body);
    out.extend_from_slice(&crc32(&body).to_be_bytes());
}

/// `channels`: 3 (RGB) o 4 (RGBA). `data`: filas seguidas de w * channels bytes.
pub fn encode(w: usize, h: usize, channels: usize, data: &[u8]) -> Vec<u8> {
    let stride = w * channels;
    let mut raw = Vec::with_capacity(h * (stride + 1));
    for y in 0..h {
        raw.push(0);
        raw.extend_from_slice(&data[y * stride..(y + 1) * stride]);
    }
    let z = miniz_oxide::deflate::compress_to_vec_zlib(&raw, 10);
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, if channels == 4 { 6 } else { 2 }, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}

/// Píxeles 0x00RRGGBB a PNG RGB.
#[allow(dead_code)]
pub fn encode_rgb32(w: usize, h: usize, px: &[u32]) -> Vec<u8> {
    let data: Vec<u8> = px.iter().flat_map(|&p| [(p >> 16) as u8, (p >> 8) as u8, p as u8]).collect();
    encode(w, h, 3, &data)
}
