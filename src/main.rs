use std::path::Path;
use std::{env, fs};

use okhsl::{Okhsl, Rgb};

const CACHE_FILE: &str = "valid_combs.bin";

#[derive(Debug)]
struct ValidCombination {
    lightness: u8,  // 0-100
    saturation: u8, // 0-100
    offset: u16,    // 0-359
}

fn gen_valid_combs(bg: &str) -> Vec<ValidCombination> {
    let bg_u8 = hex_to_rgb_u8(bg);
    let bg_rgb = hex_to_rgb(bg);
    let bg_lum = relative_luminance(bg_rgb);
    let mut valid = Vec::new();

    println!("Computing valid combinations... this takes a few seconds on the first run");

    for l in 0..=100 {
        for s in 0..=100 {
            for o in 0..360 {
                let lightness = f32::from(l) / 100.0;
                let saturation = f32::from(s) / 100.0;

                let mut all_pass = true;
                for n in 0..6 {
                    let hue_degrees = ((n as f32).mul_add(60.0, f32::from(o))) % 360.0;
                    let h = f64::from(hue_degrees / 360.0);

                    let okhsl = Okhsl { h, s: saturation, l: lightness };
                    let rgb = okhsl.to_srgb();

                    let fg_lum = relative_luminance((
                        f32::from(rgb.r) / 255.0,
                        f32::from(rgb.g) / 255.0,
                        f32::from(rgb.b) / 255.0,
                    ));
                    let wcag = wcag_contrast(bg_lum, fg_lum);
                    let apca = apca_contrast([rgb.r, rgb.g, rgb.b], bg_u8);

                    if wcag < 4.5 || apca.abs() < 32.0 {
                        all_pass = false;
                        break;
                    }
                }

                if all_pass {
                    valid.push(ValidCombination { lightness: l, saturation: s, offset: o });
                }
            }
        }
        if l % 10 == 0 {
            println!("Progress: {l}%");
        }
    }

    valid
}

fn load_or_gen_combs(bg: &str) -> Vec<ValidCombination> {
    let cache_path = format!("{CACHE_FILE}.{bg}");

    if Path::new(&cache_path).exists() {
        println!("Loading cached combinations...");
        if let Ok(data) = fs::read(&cache_path) {
            let count = data.len() / 4;
            let mut combinations = Vec::with_capacity(count);
            for chunk in data.chunks_exact(4) {
                combinations.push(ValidCombination {
                    lightness: chunk[0],
                    saturation: chunk[1],
                    offset: u16::from_le_bytes([chunk[2], chunk[3]]),
                });
            }
            println!("Loaded {} valid combinations", combinations.len());
            return combinations;
        }
    }

    let combinations = gen_valid_combs(bg);

    let mut data = Vec::with_capacity(combinations.len() * 4);
    for combo in &combinations {
        data.push(combo.lightness);
        data.push(combo.saturation);
        data.extend_from_slice(&combo.offset.to_le_bytes());
    }

    if fs::write(&cache_path, data).is_ok() {
        println!("Cached {} combinations to {}", combinations.len(), cache_path);
    }

    combinations
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut bg = String::from("000000");
    let mut saturation = 100.0;
    let mut lightness = 60.0;
    let mut offset = 0.0;
    let mut count = 6;

    let mut random_mode = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-b" | "--background" => {
                bg.clone_from(&args[i + 1]);
                i += 2;
            }
            "-s" | "--saturation" => {
                saturation = args[i + 1].parse().unwrap();
                i += 2;
            }
            "-l" | "--lightness" => {
                lightness = args[i + 1].parse().unwrap();
                i += 2;
            }
            "-o" | "--offset" => {
                offset = args[i + 1].parse().unwrap();
                i += 2;
            }
            "-c" | "--count" => {
                count = args[i + 1].parse().unwrap();
                i += 2;
            }
            "-r" | "--random" => {
                random_mode = true;
                i += 1;
            }
            "-a" | "--analyze" => {
                analyze_colorschemes();
                return;
            }
            _ => i += 1,
        }
    }

    let bg_rgb = hex_to_rgb(&bg);
    let bg_lum = relative_luminance(bg_rgb);
    let bg_u8 = hex_to_rgb_u8(&bg);

    let mut has_contrast_issue = false;

    if random_mode {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};

        let valid_combos = load_or_gen_combs(&bg);

        if valid_combos.is_empty() {
            eprintln!("No valid combinations found for this background!");
            return;
        }

        let random_state = RandomState::new();
        let mut hasher = random_state.build_hasher();
        std::time::SystemTime::now().hash(&mut hasher);
        std::process::id().hash(&mut hasher);
        let idx = (hasher.finish() as usize) % valid_combos.len();
        let combo = &valid_combos[idx];

        lightness = f32::from(combo.lightness);
        saturation = f32::from(combo.saturation);
        offset = f32::from(combo.offset);

        println!("Random mode: l={lightness} s={saturation} o={offset}\n");
    }

    let s = saturation / 100.0;
    let l = lightness / 100.0;

    let mut all_colors = Vec::new();
    for n in 0..count {
        let hue_degrees = (offset + (n as f32 * 360.0 / count as f32)) % 360.0;
        let h = f64::from(hue_degrees / 360.0);

        let okhsl = Okhsl { h, s, l };
        let rgb = okhsl.to_srgb();
        let hex = rgb_to_hex(rgb);

        all_colors.push(hex.clone());

        let fg_lum = relative_luminance((
            f32::from(rgb.r) / 255.0,
            f32::from(rgb.g) / 255.0,
            f32::from(rgb.b) / 255.0,
        ));
        let wcag = wcag_contrast(bg_lum, fg_lum);
        let apca = apca_contrast([rgb.r, rgb.g, rgb.b], bg_u8);

        let wcag_pass = if wcag >= 7.0 {
            "✅"
        } else {
            has_contrast_issue = true;
            "❌"
        };
        let apca_pass = if apca.abs() >= 50.0 {
            "✅"
        } else {
            has_contrast_issue = true;
            "❌"
        };

        let colored_hex = colorize_output(&hex, &format!("#{hex}"));
        println!("{colored_hex} | WCAG: {wcag:.2} {wcag_pass} | APCA: {apca:.0} {apca_pass}");
    }

    print_sample_text(&all_colors);

    if has_contrast_issue {
        println!("\nChange lightness and/or saturation for better contrast.");
    }
}

fn colorize_output(hex: &str, text: &str) -> String {
    format!(
        "\x1b[1m\x1b[48;2;0;0;0m\x1b[38;2;{};{};{}m{}\x1b[0m",
        u8::from_str_radix(&hex[0..2], 16).unwrap(),
        u8::from_str_radix(&hex[2..4], 16).unwrap(),
        u8::from_str_radix(&hex[4..6], 16).unwrap(),
        text
    )
}

fn hex_to_rgb(hex: &str) -> (f32, f32, f32) {
    let hex = hex.trim_start_matches('#');
    let r = f32::from(u8::from_str_radix(&hex[0..2], 16).unwrap()) / 255.0;
    let g = f32::from(u8::from_str_radix(&hex[2..4], 16).unwrap()) / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
    (r, g, f32::from(b) / 255.0)
}

fn hex_to_rgb_u8(hex: &str) -> [u8; 3] {
    let hex = hex.trim_start_matches('#');
    [
        u8::from_str_radix(&hex[0..2], 16).unwrap(),
        u8::from_str_radix(&hex[2..4], 16).unwrap(),
        u8::from_str_radix(&hex[4..6], 16).unwrap(),
    ]
}

fn rgb_to_hex(rgb: Rgb<u8>) -> String {
    format!("{:02X}{:02X}{:02X}", rgb.r, rgb.g, rgb.b)
}

fn relative_luminance(rgb: (f32, f32, f32)) -> f32 {
    let r = linearize(rgb.0);
    let g = linearize(rgb.1);
    let b = linearize(rgb.2);
    0.072_2_f32.mul_add(b, 0.212_6_f32.mul_add(r, 0.715_2 * g))
}

fn linearize(v: f32) -> f32 {
    if v <= 0.040_45 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

fn wcag_contrast(l1: f32, l2: f32) -> f32 {
    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

fn apca_contrast(fg: [u8; 3], bg: [u8; 3]) -> f64 {
    const B_EXP: f64 = 1.414;
    const R_SCALE: f64 = 1.14;
    const B_THRESH: f64 = 0.022;
    const W_OFFSET: f64 = 0.027;
    const P_IN: f64 = 0.0005;
    const P_OUT: f64 = 0.1;

    fn f_clamp(y: f64) -> f64 {
        if y >= B_THRESH { y } else { y + (B_THRESH - y).powf(B_EXP) }
    }

    fn screen_luminance(r: f64, g: f64, b: f64) -> f64 {
        b.powf(2.4)
            .mul_add(0.072_175_0, r.powf(2.4).mul_add(0.212_672_9, g.powf(2.4) * 0.715_152_2))
    }

    let fg_f = [f64::from(fg[0]) / 255.0, f64::from(fg[1]) / 255.0, f64::from(fg[2]) / 255.0];
    let bg_f = [f64::from(bg[0]) / 255.0, f64::from(bg[1]) / 255.0, f64::from(bg[2]) / 255.0];

    let fg_luma = f_clamp(screen_luminance(fg_f[0], fg_f[1], fg_f[2]));
    let bg_luma = f_clamp(screen_luminance(bg_f[0], bg_f[1], bg_f[2]));

    let s_norm = bg_luma.powf(0.56) - fg_luma.powf(0.57);
    let s_rev = bg_luma.powf(0.65) - fg_luma.powf(0.62);

    let c = if (bg_luma - fg_luma).abs() < P_IN {
        0.0
    } else if fg_luma < bg_luma {
        s_norm * R_SCALE
    } else {
        s_rev * R_SCALE
    };

    let s_apc = if c.abs() < P_OUT {
        0.0
    } else if c > 0.0 {
        c - W_OFFSET
    } else {
        c + W_OFFSET
    };

    s_apc * 100.0
}

pub fn print_sample_text(colors: &[String]) {
    let text = "Lorem ipsum dolor sit amet consectetur adipiscing elit. Quisque faucibus ex \
                sapien vitae pellentesque sem placerat. In id cursus mi pretium tellus duis \
                convallis. Tempus leo eu aenean sed diam urna tempor. Pulvinar vivamus fringilla \
                lacus nec metus bibendum egestas. Iaculis massa nisl malesuada lacinia integer \
                nunc posuere. Ut hendrerit semper vel class aptent taciti sociosqu. Ad litora \
                torquent per conubia nostra inceptos himenaeos.";

    let words: Vec<&str> = text.split_whitespace().collect();

    println!("\nBold:");
    for (i, word) in words.iter().enumerate() {
        let color = &colors[i % colors.len()];
        let (r, g, b) = parse_hex(color);
        print!("\x1b[1m\x1b[38;2;{r};{g};{b}m{word}\x1b[0m ");
    }

    println!("\n\nNormal:");
    for (i, word) in words.iter().enumerate() {
        let color = &colors[i % colors.len()];
        let (r, g, b) = parse_hex(color);
        print!("\x1b[1m\x1b[38;2;{r};{g};{b}m{word}\x1b[0m ");
    }
    println!();
}

fn parse_hex(hex: &str) -> (u8, u8, u8) {
    (
        u8::from_str_radix(&hex[0..2], 16).unwrap(),
        u8::from_str_radix(&hex[2..4], 16).unwrap(),
        u8::from_str_radix(&hex[4..6], 16).unwrap(),
    )
}

fn analyze_colorschemes() {
    let schemes = [
        ("Nord", "2E3440", vec!["bf616a", "a3be8c", "ebcb8b", "81a1c1", "b48ead", "8fbcbb"]),
        ("Dracula", "282a36", vec!["ff5555", "50fa7b", "f1fa8c", "bd93f9", "ff79c6", "8be9fd"]),
        ("Catppuccin", "1e1e2e", vec!["f38ba8", "a6e3a1", "f9e2af", "89b4fa", "cba6f7", "94e2d5"]),
        ("Gruvbox", "1d2021", vec!["fb4934", "b8bb26", "fabd2f", "83a598", "d3869b", "8ec07c"]),
        ("Rosepine", "191724", vec!["eb6f92", "31748f", "f6c177", "c4a7e7", "ebbcba", "9ccfd8"]),
    ];

    for (name, bg_hex, colors) in schemes {
        println!("\n{name} Analysis:");
        println!("Background: #{bg_hex}");
        println!("─────────────────────────────────────────────────────────────────");

        let bg_rgb = hex_to_rgb(bg_hex);
        let bg_lum = relative_luminance(bg_rgb);
        let bg_u8 = hex_to_rgb_u8(bg_hex);

        for color_hex in colors {
            let fg_rgb = hex_to_rgb(color_hex);
            let fg_u8 = hex_to_rgb_u8(color_hex);
            let fg_lum = relative_luminance(fg_rgb);

            let wcag = wcag_contrast(bg_lum, fg_lum);
            let apca = apca_contrast(fg_u8, bg_u8);

            let rgb = Rgb { r: fg_u8[0], g: fg_u8[1], b: fg_u8[2] };
            let oklab = okhsl::Oklab::from(rgb);
            let okhsl = okhsl::Okhsl::from(oklab);

            let wcag_status = if wcag >= 7.0 { "✅" } else { "❌" };
            let apca_status = if apca.abs() >= 50.0 { "✅" } else { "❌" };

            let colored_hex = format!(
                "\x1b[1m\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m#{}\x1b[0m",
                bg_u8[0],
                bg_u8[1],
                bg_u8[2], // background color
                fg_u8[0],
                fg_u8[1],
                fg_u8[2], // foreground color
                color_hex.to_uppercase()
            );
            println!(
                "{} | WCAG: {:5.2} {} | APCA: {:4.0} {} | H:{:6.1}° S:{:4.1}% L:{:4.1}%",
                colored_hex,
                wcag,
                wcag_status,
                apca,
                apca_status,
                okhsl.h * 360.0,
                okhsl.s * 100.0,
                okhsl.l * 100.0
            );
        }
    }
}
