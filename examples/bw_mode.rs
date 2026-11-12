//! Black & white / accessibility mode documentation examples.
//!
//! Generates canonical SVG outputs used in docs/src/reference/bw_mode.md.
//! Run with:
//!
//! ```bash
//! cargo run --example bw_mode
//! ```
//!
//! SVGs are written to `docs/src/assets/bw_mode/`.

use kuva::backend::svg::SvgBackend;
use kuva::plot::{BarPlot, Heatmap, LinePlot, ScatterPlot};
use kuva::render::layout::Layout;
use kuva::render::plots::Plot;
use kuva::render::render::render_multiple;

const OUT: &str = "docs/src/assets/bw_mode";

fn main() {
    std::fs::create_dir_all(OUT).expect("could not create docs/src/assets/bw_mode");

    comparison();
    patterns();
    lines();
    markers();
    colormap();

    println!("BW mode SVGs written to {OUT}/");
}

fn grouped_bar() -> BarPlot {
    BarPlot::new()
        .with_group(
            "Q1",
            vec![(18.0, "steelblue"), (12.0, "crimson"), (9.0, "seagreen")],
        )
        .with_group(
            "Q2",
            vec![(22.0, "steelblue"), (17.0, "crimson"), (14.0, "seagreen")],
        )
        .with_group(
            "Q3",
            vec![(19.0, "steelblue"), (21.0, "crimson"), (11.0, "seagreen")],
        )
        .with_group(
            "Q4",
            vec![(25.0, "steelblue"), (15.0, "crimson"), (18.0, "seagreen")],
        )
        .with_legend(vec!["Product A", "Product B", "Product C"])
        .with_stacked()
}

/// Same stacked bar chart rendered in color and in BW mode, side by side —
/// demonstrates that `.with_bw_mode()` is the only change needed.
fn comparison() {
    let plots = vec![Plot::Bar(grouped_bar())];
    let layout = Layout::auto_from_plots(&plots)
        .with_title("Color mode")
        .with_y_label("Sales (units)");
    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    std::fs::write(format!("{OUT}/comparison_color.svg"), svg).unwrap();

    let plots = vec![Plot::Bar(grouped_bar())];
    let layout = Layout::auto_from_plots(&plots)
        .with_bw_mode()
        .with_title("BW mode")
        .with_y_label("Sales (units)");
    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    std::fs::write(format!("{OUT}/comparison_bw.svg"), svg).unwrap();
}

/// A 5-series stacked bar chart in BW mode — each series gets a distinct
/// grey shade + hatch pattern combination.
fn patterns() {
    let plot = BarPlot::new()
        .with_group(
            "North",
            vec![
                (14.0, "steelblue"),
                (9.0, "crimson"),
                (11.0, "seagreen"),
                (7.0, "goldenrod"),
                (5.0, "purple"),
            ],
        )
        .with_group(
            "South",
            vec![
                (18.0, "steelblue"),
                (12.0, "crimson"),
                (8.0, "seagreen"),
                (10.0, "goldenrod"),
                (6.0, "purple"),
            ],
        )
        .with_group(
            "East",
            vec![
                (11.0, "steelblue"),
                (15.0, "crimson"),
                (13.0, "seagreen"),
                (9.0, "goldenrod"),
                (8.0, "purple"),
            ],
        )
        .with_legend(vec![
            "Widgets",
            "Gadgets",
            "Gizmos",
            "Doohickeys",
            "Thingamajigs",
        ])
        .with_stacked();

    let plots = vec![Plot::Bar(plot)];
    let layout = Layout::auto_from_plots(&plots)
        .with_bw_mode()
        .with_title("Discrete series — pattern + grey cycling")
        .with_y_label("Units");

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    std::fs::write(format!("{OUT}/patterns.svg"), svg).unwrap();
}

/// Four line series in BW mode — each cycles through a distinct dash style.
fn lines() {
    let series: Vec<(&str, Vec<(f64, f64)>)> = vec![
        (
            "Control",
            vec![(0.0, 2.0), (1.0, 2.4), (2.0, 2.1), (3.0, 2.8), (4.0, 3.0)],
        ),
        (
            "Low dose",
            vec![(0.0, 2.0), (1.0, 3.1), (2.0, 3.6), (3.0, 4.0), (4.0, 4.5)],
        ),
        (
            "Medium dose",
            vec![(0.0, 2.0), (1.0, 3.8), (2.0, 4.9), (3.0, 5.8), (4.0, 6.5)],
        ),
        (
            "High dose",
            vec![(0.0, 2.0), (1.0, 4.5), (2.0, 6.2), (3.0, 7.6), (4.0, 8.7)],
        ),
    ];

    let plots: Vec<Plot> = series
        .into_iter()
        .map(|(name, data)| {
            Plot::Line(
                LinePlot::new()
                    .with_data(data)
                    .with_color("steelblue")
                    .with_legend(name),
            )
        })
        .collect();

    let layout = Layout::auto_from_plots(&plots)
        .with_bw_mode()
        .with_title("Lines — dash-style cycling")
        .with_x_label("Time (days)")
        .with_y_label("Response");

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    std::fs::write(format!("{OUT}/lines.svg"), svg).unwrap();
}

/// Five scatter groups in BW mode — each cycles through a distinct marker shape.
fn markers() {
    let groups: Vec<(&str, Vec<(f64, f64)>)> = vec![
        ("A", vec![(1.0, 2.0), (1.4, 2.6), (1.8, 2.2), (2.1, 2.9)]),
        ("B", vec![(2.0, 4.5), (2.4, 5.0), (2.8, 4.7), (3.1, 5.3)]),
        ("C", vec![(3.0, 1.5), (3.4, 1.9), (3.8, 1.6), (4.1, 2.1)]),
        ("D", vec![(4.0, 6.0), (4.4, 6.4), (4.8, 6.1), (5.1, 6.7)]),
        ("E", vec![(5.0, 3.5), (5.4, 3.9), (5.8, 3.6), (6.1, 4.1)]),
    ];

    let plots: Vec<Plot> = groups
        .into_iter()
        .map(|(name, data)| {
            Plot::Scatter(
                ScatterPlot::new()
                    .with_data(data)
                    .with_color("crimson")
                    .with_size(6.0)
                    .with_legend(name),
            )
        })
        .collect();

    let layout = Layout::auto_from_plots(&plots)
        .with_bw_mode()
        .with_title("Scatter — marker-shape cycling")
        .with_x_label("X")
        .with_y_label("Y");

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    std::fs::write(format!("{OUT}/markers.svg"), svg).unwrap();
}

/// Heatmap in BW mode — the configured colormap is swapped for a grayscale ramp.
fn colormap() {
    let data = vec![
        vec![0.8, 0.3, 0.9, 0.2, 0.6],
        vec![0.4, 0.7, 0.1, 0.8, 0.3],
        vec![0.5, 0.9, 0.4, 0.6, 0.1],
        vec![0.2, 0.5, 0.8, 0.3, 0.7],
    ];

    let heatmap = Heatmap::new().with_data(data);

    let plots = vec![Plot::Heatmap(heatmap)];
    let layout = Layout::auto_from_plots(&plots)
        .with_bw_mode()
        .with_title("Continuous colormap — forced to grayscale");

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    std::fs::write(format!("{OUT}/colormap.svg"), svg).unwrap();
}
