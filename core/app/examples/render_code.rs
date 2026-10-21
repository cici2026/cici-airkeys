//! Renders a message as Lumen Code frames (PNG), the way the web app shows it — to film with
//! a camera in tests (an emulator's virtual scene, a printed page).
//!
//!   cargo run --release -p airkeys-app --example render_code -- message.json out-prefix [frames] [grid] [ecc]
//!
//! `grid` is the cells a side (`100`, the default) or across and down (`60x102`); `ecc` the
//! parity level (0–3, default 1).
use lumen::Renderer;
use lumen::transfer::{Sender, prepare};
use std::fs::File;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let data = std::fs::read(&args[1]).expect("a message file");
    let frames: usize = args.get(3).map_or(1, |s| s.parse().unwrap());
    let grid = args.get(4).map_or("100", |s| s.as_str());
    let (x, y) = grid.split_once('x').unwrap_or((grid, grid));
    let (x, y): (usize, usize) = (x.parse().unwrap(), y.parse().unwrap());
    let ecc: usize = args.get(5).map_or(1, |s| s.parse().unwrap());
    let prepared = prepare(&data, "airkeys-sign.json", true);
    let mut tx = Sender::new(&prepared, 1, ecc, x, y, 0x2a2a, 0).expect("a sender");
    eprintln!("payload {} B, {} B per frame", prepared.payload.len(), tx.bytes_per_frame());
    for n in 0..frames {
        let frame = tx.next_frame();
        let r = Renderer::new(&tx.layout, tx.mode, 8, 4);
        let mut px = vec![0u32; r.width * r.height];
        r.draw(&frame.cells, &mut px);
        let path = format!("{}{n}.png", args[2]);
        let mut enc = png::Encoder::new(File::create(&path).unwrap(), r.width as u32, r.height as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let bytes: Vec<u8> = px.iter().flat_map(|p| p.to_le_bytes()).collect();
        enc.write_header().unwrap().write_image_data(&bytes).unwrap();
        eprintln!("{path}: {}×{}", r.width, r.height);
    }
}
