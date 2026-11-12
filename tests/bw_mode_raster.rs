#![cfg(feature = "png")]
//! BW mode's hatch pattern overlays are SVG `<pattern>` fills the raster
//! backend can't resolve directly (`fill="url(#kuva-fp-...)"` isn't a color).
//! Before the fix, the raster backend silently skipped drawing them, so PNG
//! output showed flat grey shading with no hatch texture at all — exactly the
//! opposite of what `--bw` exists for. These tests decode real PNG output and
//! confirm the hatch ink is actually there.

mod common;

use kuva::plot::{BarPlot, DensityPlot};
use kuva::render::layout::Layout;
use kuva::render::plots::Plot;
use kuva::render::render::render_multiple;
use kuva::PngBackend;

/// Decode PNG bytes into `(width, height, rgba_pixels)`.
fn decode_png(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().expect("valid PNG");
    let mut buf = vec![0; reader.output_buffer_size().expect("known PNG size")];
    let info = reader.next_frame(&mut buf).expect("decodable PNG frame");
    buf.truncate(info.buffer_size());
    (info.width, info.height, buf)
}

fn luminance(pixels: &[u8], width: u32, x: u32, y: u32) -> u32 {
    let idx = (y * width + x) as usize * 4;
    (pixels[idx] as u32 + pixels[idx + 1] as u32 + pixels[idx + 2] as u32) / 3
}

/// 10th-to-90th-percentile luminance spread over the rectangle `[x0,x1) x
/// [y0,y1)`. A flat fill has a near-zero spread; a hatch overlay alternates
/// between the grey base and near-black ink over a large fraction of the
/// area, giving a wide spread. Percentiles (not min/max) make this robust to
/// the odd stray pixel (anti-aliased shape edge, etc.) that isn't part of
/// either the fill or the hatch.
fn luminance_spread(pixels: &[u8], width: u32, x0: u32, x1: u32, y0: u32, y1: u32) -> u32 {
    let mut lums: Vec<u32> = Vec::with_capacity(((x1 - x0) * (y1 - y0)) as usize);
    for y in y0..y1 {
        for x in x0..x1 {
            lums.push(luminance(pixels, width, x, y));
        }
    }
    lums.sort_unstable();
    let n = lums.len();
    lums[n * 9 / 10] - lums[n / 10]
}

#[test]
fn bw_bar_png_shows_hatch_texture_not_flat_grey() {
    // Gridlines disabled: they'd otherwise cross the sampled region at tick
    // values and could be mistaken for hatch texture.
    let bar = BarPlot::new()
        .with_bar("A", 10.0)
        .with_bar("B", 20.0)
        .with_bar("C", 15.0);
    let plots = vec![Plot::Bar(bar)];
    let layout = Layout::auto_from_plots(&plots)
        .with_bw_mode()
        .with_show_grid(false);
    let scene = render_multiple(plots, layout);
    let bytes = PngBackend::new().render_scene(&scene).expect("PNG render");
    common::write_test_output("test_outputs/bw_bar_raster.png", &bytes).unwrap();

    let (width, height, pixels) = decode_png(&bytes);
    // A generous interior box within the first (shortest) bar: safely clear
    // of its top edge, the baseline axis, and the plot's left/right margins.
    let x0 = width / 8;
    let x1 = width / 3;
    let y0 = height * 3 / 4;
    let y1 = height - height / 12;
    let spread = luminance_spread(&pixels, width, x0, x1, y0, y1);
    assert!(
        spread > 40,
        "a BW bar's interior should show alternating hatch/gap luminance \
         (10th-90th percentile spread {spread}), not a flat grey fill"
    );
}

#[test]
fn bw_density_png_path_fill_shows_hatch_texture() {
    // Path-based fills (density/violin/band) go through a different raster
    // code path (fill_polygon) than bars (fill_rect) — cover both.
    let density = DensityPlot::new()
        .with_data(vec![1.0, 1.5, 1.8, 2.1, 1.9, 2.0, 1.7])
        .with_filled(true);
    let plots = vec![Plot::Density(density)];
    let layout = Layout::auto_from_plots(&plots)
        .with_bw_mode()
        .with_show_grid(false);
    let scene = render_multiple(plots, layout);
    let bytes = PngBackend::new().render_scene(&scene).expect("PNG render");
    common::write_test_output("test_outputs/bw_density_raster.png", &bytes).unwrap();

    let (width, height, pixels) = decode_png(&bytes);
    // A box straddling the density curve's peak, comfortably inside the
    // filled area (well above the baseline, well below the plot top).
    let x0 = width * 3 / 8;
    let x1 = width * 5 / 8;
    let y0 = height / 2;
    let y1 = height * 3 / 4;
    let spread = luminance_spread(&pixels, width, x0, x1, y0, y1);
    assert!(
        spread > 40,
        "a BW density fill's interior should show hatch texture \
         (10th-90th percentile spread {spread}), not a flat fill"
    );
}
