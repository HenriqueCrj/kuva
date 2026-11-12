//! ColorMap swatch reference — generates a gradient bar for every built-in
//! `ColorMap` variant, grouped the same way `src/plot/colormap.rs`'s own
//! doc comments group them.
//!
//! Run:
//!   cargo run --example colormap_swatches --features full
//!
//! SVG is written to `docs/src/assets/reference/colormap_swatches.svg`.

use kuva::plot::ColorMap;

const OUT: &str = "docs/src/assets/reference";

fn main() {
    let maps: &[(&str, ColorMap)] = &[
        ("Turbo", ColorMap::Turbo),
        ("Viridis", ColorMap::Viridis),
        ("Inferno", ColorMap::Inferno),
        ("Magma", ColorMap::Magma),
        ("Plasma", ColorMap::Plasma),
        ("Cividis", ColorMap::Cividis),
        ("Warm", ColorMap::Warm),
        ("Cool", ColorMap::Cool),
        ("Cubehelix", ColorMap::Cubehelix),
        ("BlueGreen", ColorMap::BlueGreen),
        ("BluePurple", ColorMap::BluePurple),
        ("GreenBlue", ColorMap::GreenBlue),
        ("OrangeRed", ColorMap::OrangeRed),
        ("PurpleBlueGreen", ColorMap::PurpleBlueGreen),
        ("PurpleBlue", ColorMap::PurpleBlue),
        ("PurpleRed", ColorMap::PurpleRed),
        ("RedPurple", ColorMap::RedPurple),
        ("YellowGreenBlue", ColorMap::YellowGreenBlue),
        ("YellowGreen", ColorMap::YellowGreen),
        ("YellowOrangeBrown", ColorMap::YellowOrangeBrown),
        ("YellowOrangeRed", ColorMap::YellowOrangeRed),
        ("Blues", ColorMap::Blues),
        ("Greens", ColorMap::Greens),
        ("Grayscale", ColorMap::Grayscale),
        ("Oranges", ColorMap::Oranges),
        ("Purples", ColorMap::Purples),
        ("Reds", ColorMap::Reds),
        ("BrownGreen", ColorMap::BrownGreen),
        ("PinkGreen", ColorMap::PinkGreen),
        ("PurpleGreen", ColorMap::PurpleGreen),
        ("PurpleOrange", ColorMap::PurpleOrange),
        ("RedBlue", ColorMap::RedBlue),
        ("RedGrey", ColorMap::RedGrey),
        ("RedYellowBlue", ColorMap::RedYellowBlue),
        ("RedYellowGreen", ColorMap::RedYellowGreen),
        ("Spectral", ColorMap::Spectral),
        ("Rainbow", ColorMap::Rainbow),
        ("Sinebow", ColorMap::Sinebow),
    ];

    // (start_index, group_name) — matches the grouping in colormap.rs's own
    // doc comments (Sequential multi-hue perceptual/ColorBrewer, Sequential
    // single-hue, Diverging, Cyclical).
    let groups: &[(usize, &str)] = &[
        (0, "Sequential multi-hue (perceptual)"),
        (9, "Sequential multi-hue (ColorBrewer)"),
        (21, "Sequential single-hue"),
        (27, "Diverging"),
        (36, "Cyclical"),
    ];

    let pad = 20.0;
    let label_col_w = 170.0;
    let swatch_w = 420.0;
    let row_h = 26.0;
    let header_h = 56.0;
    let group_header_h = 22.0;
    let svg_w = pad * 2.0 + label_col_w + swatch_w;

    let total_rows = maps.len() as f64 * row_h + groups.len() as f64 * group_header_h;
    let svg_h = header_h + total_rows + pad;

    let mut out = String::with_capacity(256 * 1024);
    out +=
        &format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="{svg_w}" height="{svg_h}">"##);
    out += "\n";
    out += r##"  <rect width="100%" height="100%" fill="white" />"##;
    out += "\n<defs>\n";

    // One <linearGradient> per colormap, 24 stops sampled from the real
    // `ColorMap::map` function, so the rendered gradient is exactly what
    // kuva itself would draw, not a hand-approximated copy.
    const STOPS: usize = 24;
    for (name, cmap) in maps {
        out += &format!(r##"  <linearGradient id="cmap-{name}" x1="0" y1="0" x2="1" y2="0">"##);
        out += "\n";
        for i in 0..STOPS {
            let t = i as f64 / (STOPS - 1) as f64;
            let color = cmap.map(t);
            let pct = t * 100.0;
            out += &format!(r##"    <stop offset="{pct}%" stop-color="{color}" />"##);
            out += "\n";
        }
        out += "  </linearGradient>\n";
    }
    out += "</defs>\n";

    out += &format!(
        r##"  <text x="{pad}" y="28" font-size="20" font-family="sans-serif" font-weight="bold" fill="#111">ColorMap swatches — kuva</text>"##
    );
    out += "\n";
    out += &format!(
        r##"  <text x="{pad}" y="48" font-size="11" font-family="sans-serif" fill="#666">All 38 built-in variants, sampled directly from ColorMap::map(). Custom(fn) is user-defined and has no fixed swatch.</text>"##
    );
    out += "\n";

    let mut y = header_h;
    for (i, (name, _cmap)) in maps.iter().enumerate() {
        if let Some(&(_, group_name)) = groups.iter().find(|&&(start, _)| start == i) {
            let text_y = y + group_header_h * 0.7;
            out += &format!(
                r##"  <rect x="{pad}" y="{y}" width="{w}" height="{group_header_h}" fill="#eef2f7" />"##,
                w = svg_w - pad * 2.0
            );
            out += "\n";
            out += &format!(
                r##"  <text x="{x}" y="{text_y}" font-size="12" font-family="sans-serif" font-weight="bold" fill="#444">{group_name}</text>"##,
                x = pad + 4.0
            );
            out += "\n";
            y += group_header_h;
        }

        let text_y = y + row_h * 0.68;
        if i % 2 == 0 {
            out += &format!(
                r##"  <rect x="{pad}" y="{y}" width="{w}" height="{row_h}" fill="#fafbfc" />"##,
                w = svg_w - pad * 2.0
            );
            out += "\n";
        }
        out += &format!(
            r##"  <text x="{x}" y="{text_y}" font-size="12" font-family="sans-serif" fill="#333">{name}</text>"##,
            x = pad + 4.0
        );
        out += "\n";

        let bar_x = pad + label_col_w;
        let bar_y = y + row_h * 0.15;
        let bar_h = row_h * 0.7;
        out += &format!(
            r##"  <rect x="{bar_x}" y="{bar_y}" width="{swatch_w}" height="{bar_h}" fill="url(#cmap-{name})" stroke="#ccc" stroke-width="0.5" />"##
        );
        out += "\n";

        y += row_h;
    }

    out += "</svg>\n";
    std::fs::create_dir_all(OUT).expect("could not create docs/src/assets/reference");
    let path = format!("{OUT}/colormap_swatches.svg");
    std::fs::write(&path, &out).unwrap_or_else(|e| panic!("failed to write {path}: {e}"));
    println!("Written: colormap_swatches.svg  ({} colormaps)", maps.len());
}
