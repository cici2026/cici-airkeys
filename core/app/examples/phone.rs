//! The phone app without a phone: drives the real `AirkeysCore` — open a wallet, show its
//! accounts or sign a request — and films the code it shows into a camera video (.y4m) that
//! Chrome can use as its webcam (`--use-file-for-fake-video-capture`). Prints the code's JSON.
//!
//!   AIRKEYS_PHRASE="…" cargo run --release -p airkeys-app --example phone -- accounts out.y4m
//!   AIRKEYS_PHRASE="…" cargo run --release -p airkeys-app --example phone -- sign request.json out.y4m
use airkeys::ops::{Action, RadioState};
use airkeys::runtime::{AirkeysCore, CodeFrame};
use airkeys::view::Screen;
use std::io::Write;

const W: usize = 1280;
const H: usize = 720;
/// The code's area on a 1080×2400 phone, from the title to the buttons.
const AREA: (u32, u32) = (1080, 1800);

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let phrase = std::env::var("AIRKEYS_PHRASE").expect("set AIRKEYS_PHRASE");
    let core = AirkeysCore::new();
    let off = RadioState::Off;
    core.dispatch(Action::Radios { wifi: off, bluetooth: off, cellular: off, network: off });
    core.dispatch(Action::Begin);
    core.dispatch(Action::PhraseInput { text: phrase });
    core.dispatch(Action::PhraseDone);
    core.dispatch(Action::PassphraseInput {
        passphrase: std::env::var("AIRKEYS_PASSPHRASE").unwrap_or_default(),
    });
    let view = core.dispatch(Action::Open).view;
    assert!(matches!(view.screen, Screen::Home { .. }), "the wallet did not open: {:?}", view.screen);

    let (mode, out) = (args[0].as_str(), args.last().unwrap());
    let screen = match mode {
        "accounts" => {
            core.dispatch(Action::Connect);
            core.dispatch(Action::ShowAccounts).view.screen
        }
        "sign" => {
            core.dispatch(Action::Scan);
            let request = std::fs::read(&args[1]).expect("a request file");
            match core.deliver(request).view.screen {
                Screen::Review { .. } => core.dispatch(Action::Sign).view.screen,
                other => other,
            }
        }
        _ => panic!("accounts | sign <request.json>"),
    };
    let Screen::Code { json, .. } = screen else {
        eprintln!("the phone shows no code: {screen:?}");
        std::process::exit(2);
    };
    println!("{json}");

    // the phone's screen filmed at 30 fps while its code plays at 10: three captures per frame
    let mut f = std::fs::File::create(out).unwrap();
    writeln!(f, "YUV4MPEG2 W{W} H{H} F30:1 Ip A1:1 C420jpeg").unwrap();
    for _ in 0..30 {
        let frame = core.code_frame(AREA.0, AREA.1).expect("a code is shown");
        let yuv = film(&frame);
        for _ in 0..3 {
            f.write_all(b"FRAME\n").unwrap();
            f.write_all(&yuv).unwrap();
        }
    }
}

/// The code scaled up to fill most of a 1280×720 frame, on a dark surround, as Y′CbCr 4:2:0.
fn film(c: &CodeFrame) -> Vec<u8> {
    let (cw, ch) = (c.width as usize, c.height as usize);
    let s = (H * 85 / 100) / ch.max(cw);
    let (ox, oy) = ((W - cw * s) / 2, (H - ch * s) / 2);
    let rgb = |x: usize, y: usize| -> [f32; 3] {
        if x >= ox && y >= oy && (x - ox) / s < cw && (y - oy) / s < ch {
            let i = ((y - oy) / s * cw + (x - ox) / s) * 4;
            [c.rgba[i] as f32, c.rgba[i + 1] as f32, c.rgba[i + 2] as f32]
        } else {
            [20.0, 22.0, 26.0]
        }
    };
    let mut out = vec![0u8; W * H * 3 / 2];
    for y in 0..H {
        for x in 0..W {
            let [r, g, b] = rgb(x, y);
            out[y * W + x] = (0.299 * r + 0.587 * g + 0.114 * b) as u8;
        }
    }
    let (u0, v0) = (W * H, W * H + W * H / 4);
    for y in 0..H / 2 {
        for x in 0..W / 2 {
            let [r, g, b] = rgb(x * 2, y * 2);
            out[u0 + y * W / 2 + x] = (128.0 - 0.168736 * r - 0.331264 * g + 0.5 * b).clamp(0.0, 255.0) as u8;
            out[v0 + y * W / 2 + x] = (128.0 + 0.5 * r - 0.418688 * g - 0.081312 * b).clamp(0.0, 255.0) as u8;
        }
    }
    out
}
