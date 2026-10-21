//! Reads the Lumen Code in screenshots (PNG) — of the phone app, or the web app — and prints
//! the message they carry. Feed it several screenshots taken in quick succession: each holds
//! one frame of the code, and a message may need more than one.
//!
//!   cargo run --release -p airkeys-app --example read_screens -- shot-*.png
use lumen::scan::{Image, Scanner};
use lumen::transfer::{Finish, Receiver};
use std::fs::File;

fn main() {
    let mut scanner = Scanner::new();
    let mut rx = Receiver::new();
    let (mut frames, mut found) = (0, 0);
    for path in std::env::args().skip(1) {
        let decoder = png::Decoder::new(std::io::BufReader::new(File::open(&path).expect("a PNG")));
        let mut reader = decoder.read_info().expect("a PNG");
        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buf).expect("a PNG frame");
        let rgba: Vec<u8> = match info.color_type {
            png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
            png::ColorType::Rgb => {
                buf[..info.buffer_size()].chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect()
            }
            other => panic!("{path}: unsupported colour type {other:?}"),
        };
        frames += 1;
        let res = scanner
            .scan(&Image { width: info.width as usize, height: info.height as usize, rgba: &rgba }, None);
        if res.ok {
            found += 1;
        }
        let ok = res.codewords.iter().filter(|c| c.ok).count();
        eprintln!(
            "{path}: {} ({} of {} codewords){}",
            if res.ok { "code found" } else { res.reason },
            ok,
            res.codewords.len(),
            res.stats.as_ref().map_or(String::new(), |s| format!(", {:.1} px per cell", s.cell_px))
        );
        for cw in res.codewords.iter().filter(|c| c.ok) {
            rx.push(cw.bytes.as_ref().unwrap());
        }
        if let Some(Finish::File { name, bytes }) = rx.try_finish() {
            eprintln!(
                "complete after {frames} screenshots ({found} with a code): {name}, {} bytes",
                bytes.len()
            );
            println!("{}", String::from_utf8_lossy(&bytes));
            return;
        }
    }
    eprintln!("not complete: {:?}", rx.progress().map(|p| (p.received, p.needed)));
    std::process::exit(1);
}
