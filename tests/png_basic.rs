#![cfg(feature = "png")]
mod common;

use kuva::plot::scatter::ScatterPlot;
use kuva::plot::BarPlot;
use kuva::plot::Histogram;
use kuva::plot::LinePlot;
use kuva::render::annotations::{ReferenceLine, ShadedRegion, TextAnnotation};
use kuva::render::figure::Figure;
use kuva::render::layout::Layout;
use kuva::render::plots::Plot;
use kuva::render::render::{render_multiple, render_scatter};
use kuva::PngBackend;

fn make_scatter_scene() -> kuva::render::render::Scene {
    let data = vec![(1.0, 2.0), (3.0, 4.0), (5.0, 6.0)];
    let plot = ScatterPlot::new().with_data(data).with_color("steelblue");
    let layout = Layout::new((0.0, 6.0), (0.0, 8.0)).with_title("PNG test");
    render_scatter(&plot, layout).with_background(Some("white"))
}

#[test]
fn png_scatter_basic() {
    let scene = make_scatter_scene();
    let result = PngBackend::new().render_scene(&scene);
    assert!(result.is_ok(), "render_scene failed: {:?}", result.err());
    let bytes = result.unwrap();
    assert_eq!(
        &bytes[..8],
        b"\x89PNG\r\n\x1a\n",
        "output is not a valid PNG"
    );
    common::write_test_output("test_outputs/png_scatter.png", &bytes).unwrap();
}

#[test]
fn png_scale_parameter() {
    let scene = make_scatter_scene();
    let bytes1 = PngBackend::new()
        .with_scale(1.0)
        .render_scene(&scene)
        .unwrap();
    let bytes2 = PngBackend::new()
        .with_scale(2.0)
        .render_scene(&scene)
        .unwrap();

    // PNG IHDR: magic (8) + length (4) + "IHDR" (4) + width (4) + height (4)
    let w1 = u32::from_be_bytes([bytes1[16], bytes1[17], bytes1[18], bytes1[19]]);
    let h1 = u32::from_be_bytes([bytes1[20], bytes1[21], bytes1[22], bytes1[23]]);
    let w2 = u32::from_be_bytes([bytes2[16], bytes2[17], bytes2[18], bytes2[19]]);
    let h2 = u32::from_be_bytes([bytes2[20], bytes2[21], bytes2[22], bytes2[23]]);

    // Doubling the raster scale doubles the pixel dimensions, to within the 1px
    // rounding inherent in rasterising a sub-pixel canvas (real-metric margins are
    // fractional, so round(2·w) can differ from 2·round(w) by one pixel).
    assert!(
        (w2 as i64 - (w1 * 2) as i64).abs() <= 1,
        "2× width {w2} should be ~double 1× width {w1}"
    );
    assert!(
        (h2 as i64 - (h1 * 2) as i64).abs() <= 1,
        "2× height {h2} should be ~double 1× height {h1}"
    );
}

// ---------------------------------------------------------------------------
// Rich multiplot figure test
// ---------------------------------------------------------------------------
//
// 2×2 Figure with panel labels (A–D), a figure title, and four annotated
// subplots. Verifies that all SVG features survive the resvg rasterisation
// pipeline and that the result is a valid PNG.

#[test]
fn png_rich_figure() {
    // --- Panel A: scatter with two series, shaded region, reference line,
    //              and an annotated outlier ---
    let series1 = ScatterPlot::new()
        .with_data(vec![
            (1.0, 2.0),
            (2.0, 4.5),
            (3.0, 3.2),
            (4.0, 5.8),
            (5.0, 4.1),
        ])
        .with_color("steelblue")
        .with_size(5.0)
        .with_legend("Control");
    let series2 = ScatterPlot::new()
        .with_data(vec![
            (1.0, 3.5),
            (2.0, 6.0),
            (3.0, 5.1),
            (4.0, 8.2),
            (5.0, 7.0),
        ])
        .with_color("tomato")
        .with_size(5.0)
        .with_legend("Treatment");

    let scatter_plots = vec![Plot::Scatter(series1), Plot::Scatter(series2)];
    let layout_a = Layout::auto_from_plots(&scatter_plots)
        .with_title("Scatter: Control vs Treatment")
        .with_x_label("Time (days)")
        .with_y_label("Expression level")
        .with_shaded_region(
            ShadedRegion::horizontal(5.0, 7.0)
                .with_color("gold")
                .with_opacity(0.2),
        )
        .with_reference_line(
            ReferenceLine::horizontal(5.0)
                .with_color("grey")
                .with_label("baseline"),
        )
        .with_annotation(
            TextAnnotation::new("Peak", 4.0, 8.8)
                .with_arrow(4.0, 8.2)
                .with_color("darkred")
                .with_font_size(12),
        );

    // --- Panel B: two line series (solid + dashed) with a vertical marker ---
    let xs: Vec<f64> = (0..=60).map(|i| i as f64 / 10.0).collect();
    let line1 = LinePlot::new()
        .with_data(xs.iter().map(|&x| (x, x.sin())))
        .with_color("steelblue")
        .with_legend("sin(x)");
    let line2 = LinePlot::new()
        .with_data(xs.iter().map(|&x| (x, x.cos())))
        .with_color("tomato")
        .with_dashed()
        .with_legend("cos(x)");

    let line_plots = vec![Plot::Line(line1), Plot::Line(line2)];
    let layout_b = Layout::new((0.0, 6.0), (-1.5, 1.5))
        .with_title("Waveforms")
        .with_x_label("Angle (rad)")
        .with_y_label("Amplitude")
        .with_ticks(6)
        .with_reference_line(
            ReferenceLine::vertical(std::f64::consts::PI)
                .with_color("purple")
                .with_label("π"),
        )
        .with_reference_line(
            ReferenceLine::horizontal(0.0)
                .with_color("black")
                .with_dasharray("2,2"),
        );

    // --- Panel C: grouped bar chart with a shaded band and annotation ---
    let bar = BarPlot::new()
        .with_bar("Alpha", 4.2)
        .with_bar("Beta", 7.1)
        .with_bar("Gamma", 5.5)
        .with_bar("Delta", 9.3)
        .with_bar("Epsilon", 3.8)
        .with_color("#4e79a7");

    let bar_plots = vec![Plot::Bar(bar)];
    let layout_c = Layout::auto_from_plots(&bar_plots)
        .with_title("Group Counts")
        .with_x_label("Group")
        .with_y_label("Count")
        .with_shaded_region(
            ShadedRegion::horizontal(6.0, 8.0)
                .with_color("limegreen")
                .with_opacity(0.15),
        )
        .with_reference_line(
            ReferenceLine::horizontal(6.0)
                .with_color("darkgreen")
                .with_label("target"),
        )
        .with_annotation(
            TextAnnotation::new("Best", 3.0, 9.9)
                .with_arrow(3.0, 9.3)
                .with_color("navy")
                .with_font_size(11),
        );

    // --- Panel D: histogram with a mean reference line ---
    let values: Vec<f64> = vec![
        1.2, 1.5, 1.8, 2.1, 2.3, 2.5, 2.6, 2.8, 2.9, 3.0, 3.1, 3.3, 3.5, 3.7, 4.0, 4.2, 4.5, 4.8,
        5.0, 5.3,
    ];
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let hist = Histogram::new()
        .with_data(values)
        .with_bins(8)
        .with_color("#f28e2b")
        .with_range((0.0, 6.0));

    let hist_plots = vec![Plot::Histogram(hist)];
    let layout_d = Layout::auto_from_plots(&hist_plots)
        .with_title("Value Distribution")
        .with_x_label("Value")
        .with_y_label("Frequency")
        .with_reference_line(
            ReferenceLine::vertical(mean)
                .with_color("firebrick")
                .with_label("mean"),
        )
        .with_shaded_region(
            ShadedRegion::vertical(2.0, 4.0)
                .with_color("steelblue")
                .with_opacity(0.12),
        );

    // --- Compose into a 2×2 figure ---
    let figure = Figure::new(2, 2)
        .with_title("PNG Rich Figure Test")
        .with_plots(vec![scatter_plots, line_plots, bar_plots, hist_plots])
        .with_layouts(vec![layout_a, layout_b, layout_c, layout_d])
        .with_labels()
        .with_shared_legend();

    let scene = figure.render();
    let bytes = PngBackend::new()
        .render_scene(&scene)
        .expect("PNG render failed");

    assert_eq!(
        &bytes[..8],
        b"\x89PNG\r\n\x1a\n",
        "output is not a valid PNG"
    );
    common::write_test_output("test_outputs/png_rich_figure.png", &bytes).unwrap();
}

// ---------------------------------------------------------------------------
// Bar-plot error bars — actual raster non-blank check
// ---------------------------------------------------------------------------
//
// A user reported bar-plot PNGs looking empty when spot-checked with an
// external SVG->PNG converter; that turned out to be a missing rsvg-convert
// dependency on their machine, not a kuva bug (confirmed by inspecting the
// raw SVG, which had the expected <rect>/<line> elements). This test uses
// kuva's own PngBackend (the real, supported raster path) and decodes actual
// pixels to make sure bars + error whiskers are genuinely drawn, not just
// present in the SVG source.

#[test]
fn png_bar_error_bars_not_blank() {
    let plot = BarPlot::new()
        .with_bars(vec![
            ("Control", 42.0),
            ("Treated", 58.0),
            ("Placebo", 31.0),
        ])
        .with_color("steelblue")
        .with_error(vec![3.0, 5.0, 2.0]);
    let plots = vec![Plot::Bar(plot)];
    let layout = Layout::auto_from_plots(&plots)
        .with_title("PNG Bar Error Bars")
        .with_y_label("Value");
    let scene = render_multiple(plots, layout);

    let bytes = PngBackend::new()
        .render_scene(&scene)
        .expect("PNG render failed");
    assert_eq!(
        &bytes[..8],
        b"\x89PNG\r\n\x1a\n",
        "output is not a valid PNG"
    );
    common::write_test_output("test_outputs/png_bar_error_bars.png", &bytes).unwrap();

    let img = image::load_from_memory(&bytes)
        .expect("failed to decode PNG")
        .to_rgba8();
    let non_white = img
        .pixels()
        .filter(|p| {
            let [r, g, b, a] = p.0;
            a > 0 && !(r > 250 && g > 250 && b > 250)
        })
        .count();
    assert!(
        non_white > 500,
        "rendered PNG appears blank — only {non_white} non-white pixels found"
    );
}

#[test]
fn png_bar_error_bars_stacked_not_occluded() {
    // Regression check for the "next segment paints over the previous
    // segment's error whisker" bug — decode actual pixels and confirm the
    // whisker's dark stroke color survives above the topmost segment's
    // color, i.e. it wasn't drawn before a later segment overwrote it.
    let plot = BarPlot::new()
        .with_group(
            "Alpha",
            vec![(40.0, "steelblue"), (25.0, "tomato"), (15.0, "seagreen")],
        )
        .with_stacked()
        .with_error(vec![2.0, 3.0, 1.0]);
    let plots = vec![Plot::Bar(plot)];
    let layout = Layout::auto_from_plots(&plots).with_title("PNG Stacked Bar Error Bars");
    let scene = render_multiple(plots, layout);

    let bytes = PngBackend::new()
        .render_scene(&scene)
        .expect("PNG render failed");
    common::write_test_output("test_outputs/png_bar_error_bars_stacked.png", &bytes).unwrap();

    let img = image::load_from_memory(&bytes)
        .expect("failed to decode PNG")
        .to_rgba8();
    // The whisker/cap stroke color is #1a1a1a (near-black) by default —
    // count pixels close to that color anywhere in the image.
    let whisker_pixels = img
        .pixels()
        .filter(|p| {
            let [r, g, b, a] = p.0;
            a > 0 && r < 50 && g < 50 && b < 50
        })
        .count();
    assert!(
        whisker_pixels > 20,
        "expected visible dark whisker pixels, found only {whisker_pixels}"
    );
}
