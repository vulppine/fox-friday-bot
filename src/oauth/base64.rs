// based off some code i did for the cryptopals challenge some time ago
// i haven't completed it yet; don't ask for the full repo until i do

const INDEX: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn bytes_to_base64(mut bytes: Vec<u8>) -> String {
    let mut result = String::new();
    let mut padding = 0;

    while bytes.len() % 3 != 0 {
        bytes.push(b'0');
        padding += 1;
    }

    let chunked_bytes = bytes.chunks(3)
        .map(|b| {
            let mut r: u32 = 0x00;
            r += (b[0] as u32) << 0x10;
            r += (b[1] as u32) << 0x08;
            r += b[2] as u32;

            r
        })
        .map(|mut b| {
            let mut r: Vec<u8> = Vec::new();
            while b > 0 {
                r.insert(0, (b & !((b >> 6) << 6)) as u8);
                b >>= 6;
            }

            r
        })
        .flatten()
        .collect::<Vec<u8>>();

    chunked_bytes
        .iter()
        .for_each(|b| result.push(INDEX[*b as usize] as char));

    result.truncate(result.len() - padding);
    while padding > 0 {
        result.push('=');
        padding -= 1;
    }

    result
}
