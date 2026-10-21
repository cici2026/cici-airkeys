//! What a scan costs on a 1280×960 camera frame with no code in it — empty, busy
//! (a checkerboard: finder-like everywhere) and noisy — on this machine.
use lumen::scan::{Scanner, YuvImage};
use std::time::Instant;

type Scene = (&'static str, Box<dyn Fn(usize, usize) -> u8>);

fn main() {
    let (w, h) = (1280usize, 960usize);
    let cbcr = vec![128u8; w * h / 2];
    let scenes: [Scene; 3] = [
        ("grey", Box::new(|_, _| 128)),
        ("checkerboard", Box::new(|x, y| if (x / 24 + y / 24) % 2 == 0 { 20 } else { 230 })),
        ("noise", Box::new(|x, y| ((x * 7919 + y * 104729) % 251) as u8)),
    ];
    for (name, f) in scenes {
        let y: Vec<u8> = (0..w * h).map(|i| f(i % w, i / w)).collect();
        let mut s = Scanner::new();
        let n = 20;
        let t = Instant::now();
        for _ in 0..n {
            s.scan_yuv(&YuvImage { width: w, height: h, y: &y, cbcr: &cbcr }, None, None);
        }
        println!("{name}: {:.1} ms per scan", t.elapsed().as_secs_f64() * 1000.0 / n as f64);
    }
}
