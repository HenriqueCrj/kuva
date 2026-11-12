//! Pareto chart documentation examples.
//!
//! Generates canonical SVG outputs used in the kuva documentation.
//! Run with:
//!
//! ```bash
//! cargo run --example pareto
//! ```
//!
//! SVGs are written to `docs/src/assets/pareto/`.

use kuva::backend::svg::SvgBackend;
use kuva::plot::pareto::ParetoPlot;
use kuva::render::layout::Layout;
use kuva::render::plots::Plot;
use kuva::render::render::render_multiple;

const OUT: &str = "docs/src/assets/pareto";

fn main() {
    std::fs::create_dir_all(OUT).expect("could not create docs/src/assets/pareto");

    basic();
    styled();
    bucketed();
    horizontal();

    println!("Pareto SVGs written to {OUT}/");
}

/// Basic Pareto chart — error categories, default 80% threshold line.
fn basic() {
    let pareto = ParetoPlot::new().with_categories(vec![
        ("Missing field", 42.0),
        ("Typo", 31.0),
        ("Timeout", 18.0),
        ("Bad format", 12.0),
        ("Duplicate entry", 9.0),
        ("Other", 6.0),
    ]);

    let plots = vec![Plot::Pareto(pareto)];
    let layout = Layout::auto_from_plots(&plots)
        .with_title("Support Ticket Error Categories")
        .with_y_label("Count");

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    std::fs::write(format!("{OUT}/basic.svg"), svg).unwrap();
}

/// Styled Pareto chart — custom colors, custom threshold, cumulative labels, legend.
fn styled() {
    let pareto = ParetoPlot::new()
        .with_categories(vec![
            ("Missing field", 42.0),
            ("Typo", 31.0),
            ("Timeout", 18.0),
            ("Bad format", 12.0),
            ("Duplicate entry", 9.0),
            ("Other", 6.0),
        ])
        .with_color("seagreen")
        .with_line_color("darkorange")
        .with_threshold(90.0)
        .with_cumulative_labels(true)
        .with_legend("Count", "Cumulative %");

    let plots = vec![Plot::Pareto(pareto)];
    let layout = Layout::auto_from_plots(&plots)
        .with_title("Support Ticket Error Categories")
        .with_y_label("Count");

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    std::fs::write(format!("{OUT}/styled.svg"), svg).unwrap();
}

/// Long-tail categories collapsed into one stacked "Other" bar, decoded via
/// per-segment legend entries.
fn bucketed() {
    let pareto = ParetoPlot::new()
        .with_categories(vec![
            ("Missing field", 42.0),
            ("Typo", 31.0),
            ("Timeout", 18.0),
            ("Bad format", 12.0),
            ("Duplicate entry", 9.0),
            ("Wrong encoding", 7.0),
            ("Network blip", 5.0),
            ("Other misc", 3.0),
        ])
        .with_max_categories(5);

    let plots = vec![Plot::Pareto(pareto)];
    let layout = Layout::auto_from_plots(&plots)
        .with_title("Long Tail Collapsed into \"Other\"")
        .with_y_label("Count");

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    std::fs::write(format!("{OUT}/bucketed.svg"), svg).unwrap();
}

/// Horizontal mode — categories on Y, values on X, cumulative-% line on a
/// secondary X-axis drawn on top.
fn horizontal() {
    let pareto = ParetoPlot::new()
        .with_categories(vec![
            ("Missing field", 42.0),
            ("Typo", 31.0),
            ("Timeout", 18.0),
            ("Bad format", 12.0),
            ("Duplicate entry", 9.0),
            ("Other", 6.0),
        ])
        .with_horizontal(true)
        .with_cumulative_labels(true);

    let plots = vec![Plot::Pareto(pareto)];
    let layout = Layout::auto_from_plots(&plots)
        .with_title("Support Ticket Error Categories")
        .with_x_label("Count");

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    std::fs::write(format!("{OUT}/horizontal.svg"), svg).unwrap();
}
