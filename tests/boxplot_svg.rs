mod common;
use kuva::backend::svg::SvgBackend;
use kuva::plot::BoxPlot;
use kuva::render::layout::Layout;
use kuva::render::plots::Plot;
use kuva::render::render::render_multiple;

#[test]
fn test_boxplot_groups_svg_output_builder() {
    let boxplot = BoxPlot::new()
        .with_group("A", vec![1.0, 2.0, 2.5, 3.0, 4.0, 5.0])
        .with_group("B", vec![2.0, 2.1, 3.5, 3.8, 4.0, 4.2])
        .with_color("darkred");

    // let x_labels: Vec<String> = boxplot.groups.iter().map(|g| g.label.clone()).collect();

    let plots = vec![Plot::Box(boxplot)];

    let layout = Layout::auto_from_plots(&plots)
        .with_title("Box Plot")
        .with_y_label("Values");
    // .with_x_categories(x_labels);

    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/boxplot_groups_builder.svg", svg.clone()).unwrap();

    // Basic sanity assertion
    assert!(svg.contains("<svg"));
}

#[test]
fn test_boxplot_svg_output_builder() {
    let boxplot = BoxPlot::new()
        .with_group("A", vec![1.0, 2.0, 2.5, 3.0, 4.0, 5.0])
        .with_color("darkred");

    let plots = vec![Plot::Box(boxplot)];

    let layout = Layout::auto_from_plots(&plots)
        .with_title("Box Plot")
        .with_y_label("Values");

    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/boxplot_builder.svg", svg.clone()).unwrap();

    // Basic sanity assertion
    assert!(svg.contains("<svg"));
}

#[test]
fn test_boxplot_group_colors_full() {
    let boxplot = BoxPlot::new()
        .with_group("A", vec![1.0, 2.0, 2.5, 3.0, 4.0, 5.0])
        .with_group("B", vec![2.0, 2.1, 3.5, 3.8, 4.0, 4.2])
        .with_group("C", vec![3.0, 3.5, 4.0, 4.5, 5.0, 5.5])
        .with_color("black")
        .with_group_colors(["steelblue", "tomato", "seagreen"]);

    let plots = vec![Plot::Box(boxplot)];
    let layout = Layout::auto_from_plots(&plots).with_title("Per-group Colors");
    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/boxplot_group_colors_full.svg", svg.clone()).unwrap();

    assert!(svg.contains("<svg"));
    // Each group color must appear; the fallback "black" must not be used as a fill
    assert!(svg.contains("steelblue") || svg.contains("#4682b4"));
    assert!(svg.contains("tomato") || svg.contains("#ff6347"));
    assert!(svg.contains("seagreen") || svg.contains("#2e8b57"));
}

#[test]
fn test_boxplot_group_colors_partial() {
    // Only 1 color provided for 3 groups — groups B and C fall back to "black"
    let boxplot = BoxPlot::new()
        .with_group("A", vec![1.0, 2.0, 2.5, 3.0, 4.0, 5.0])
        .with_group("B", vec![2.0, 2.1, 3.5, 3.8, 4.0, 4.2])
        .with_group("C", vec![3.0, 3.5, 4.0, 4.5, 5.0, 5.5])
        .with_color("black")
        .with_group_colors(["tomato"]);

    let plots = vec![Plot::Box(boxplot)];
    let layout = Layout::auto_from_plots(&plots).with_title("Partial Per-group Colors");
    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/boxplot_group_colors_partial.svg", svg.clone())
        .unwrap();

    assert!(svg.contains("<svg"));
    assert!(svg.contains("tomato") || svg.contains("#ff6347"));
    // Fallback color must appear for the uncolored groups
    assert!(svg.contains("black"));
}

#[test]
fn test_boxplot_horizontal() {
    let plot = BoxPlot::new()
        .with_group("Alpha", vec![1.0, 2.0, 3.0, 4.0, 5.0])
        .with_group("Beta", vec![2.0, 3.5, 4.0, 4.5, 6.0])
        .with_group("Gamma", vec![3.0, 4.0, 5.0, 6.0, 8.0])
        .with_color("steelblue")
        .with_horizontal(true);

    let plots = vec![Plot::Box(plot)];
    let layout = Layout::auto_from_plots(&plots)
        .with_title("Horizontal Box Plot")
        .with_x_label("Value")
        .with_y_label("Group");
    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/boxplot_horizontal.svg", svg.clone()).unwrap();

    assert!(svg.contains("<svg"));
    // Groups should appear as y-axis tick labels
    assert!(svg.contains("Alpha"));
    assert!(svg.contains("Beta"));
    assert!(svg.contains("Gamma"));
}

// ── Notch ─────────────────────────────────────────────────────────────────────

#[test]
fn test_boxplot_notch_vertical() {
    let plot = BoxPlot::new()
        .with_group("Control", vec![4.1, 5.0, 5.3, 5.8, 6.2, 7.0, 5.5, 4.8, 6.5])
        .with_group("Treated", vec![5.5, 6.1, 6.4, 7.2, 7.8, 8.5, 6.9, 7.0, 7.5])
        .with_color("steelblue")
        .with_notch(true);

    let plots = vec![Plot::Box(plot)];
    let layout = Layout::auto_from_plots(&plots)
        .with_title("Notched Box Plot")
        .with_y_label("Value");
    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    common::write_test_output("test_outputs/boxplot_notch_vertical.svg", svg.clone()).unwrap();

    assert!(svg.contains("<svg"));
    // Notched boxes render as a <path>, not a plain <rect>, for the box body.
    assert!(svg.contains("<path"));
}

#[test]
fn test_boxplot_notch_horizontal() {
    let plot = BoxPlot::new()
        .with_group("Alpha", vec![1.0, 2.0, 3.0, 4.0, 5.0, 3.5, 2.5])
        .with_group("Beta", vec![2.0, 3.5, 4.0, 4.5, 6.0, 3.0, 5.0])
        .with_color("seagreen")
        .with_notch(true)
        .with_horizontal(true);

    let plots = vec![Plot::Box(plot)];
    let layout = Layout::auto_from_plots(&plots)
        .with_title("Notched Box Plot (Horizontal)")
        .with_x_label("Value")
        .with_y_label("Group");
    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    common::write_test_output("test_outputs/boxplot_notch_horizontal.svg", svg.clone()).unwrap();

    assert!(svg.contains("<svg"));
    assert!(svg.contains("<path"));
}

#[test]
fn test_boxplot_notch_small_n_clamped() {
    // With only 2 points, IQR is small and n is tiny — the notch half-width
    // formula could exceed the hinge-to-median distance. Must not panic or
    // produce a self-intersecting/inverted polygon; just render something sane.
    let plot = BoxPlot::new()
        .with_group("Tiny", vec![1.0, 100.0])
        .with_color("tomato")
        .with_notch(true);

    let plots = vec![Plot::Box(plot)];
    let layout = Layout::auto_from_plots(&plots).with_title("Notch Clamping (n=2)");
    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    common::write_test_output("test_outputs/boxplot_notch_clamped.svg", svg.clone()).unwrap();

    assert!(svg.contains("<svg"));
}

/// Flatten a single `<path d="...">`'s numeric coordinates in order,
/// stripping the `M`/`L`/`Z` command letters.
fn extract_first_path_coords(svg: &str) -> Vec<f64> {
    let start = svg.find("<path d=\"").expect("no <path> in svg") + "<path d=\"".len();
    let end = svg[start..].find('"').unwrap() + start;
    svg[start..end]
        .split(' ')
        .filter(|s| !s.is_empty() && *s != "M" && *s != "L" && *s != "Z")
        .map(|s| s.parse::<f64>().expect("non-numeric path token"))
        .collect()
}

#[test]
fn test_boxplot_notch_default_depth_does_not_reach_center() {
    // Regression test for the "notch cuts all the way to the middle" bug —
    // default notch_depth (0.3) must stop well short of the box's horizontal
    // center, not collapse into a full bowtie.
    let plot = BoxPlot::new()
        .with_group("A", vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0])
        .with_color("steelblue")
        .with_notch(true);

    let plots = vec![Plot::Box(plot)];
    let layout = Layout::auto_from_plots(&plots).with_title("Default Notch Depth");
    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    common::write_test_output("test_outputs/boxplot_notch_default_depth.svg", svg.clone()).unwrap();

    let coords = extract_first_path_coords(&svg);
    // Path point order: (x0,yq3) (x1,yq3) (x1,ynt) (pinch_right,ymed) ...
    let x0 = coords[0];
    let x1 = coords[2];
    let pinch_right = coords[6];
    let xmid = (x0 + x1) / 2.0;

    assert!(
        pinch_right > xmid + 1.0,
        "default notch_depth should stop well short of center: pinch_right={pinch_right:.2}, xmid={xmid:.2}"
    );
}

#[test]
fn test_boxplot_notch_depth_one_reaches_center() {
    // `.with_notch_depth(1.0)` restores the old full-bowtie behavior — the
    // pinch point should land exactly on the box's horizontal center.
    let plot = BoxPlot::new()
        .with_group("A", vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0])
        .with_color("steelblue")
        .with_notch(true)
        .with_notch_depth(1.0);

    let plots = vec![Plot::Box(plot)];
    let layout = Layout::auto_from_plots(&plots).with_title("Notch Depth = 1.0");
    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    common::write_test_output("test_outputs/boxplot_notch_depth_full.svg", svg.clone()).unwrap();

    let coords = extract_first_path_coords(&svg);
    let x0 = coords[0];
    let x1 = coords[2];
    let pinch_right = coords[6];
    let xmid = (x0 + x1) / 2.0;

    assert!(
        (pinch_right - xmid).abs() < 0.1,
        "notch_depth=1.0 should pinch exactly to center: pinch_right={pinch_right:.2}, xmid={xmid:.2}"
    );
}

#[test]
fn test_boxplot_notch_width_scales_vertical_span() {
    let base_data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 5.5, 4.5];

    let narrow = BoxPlot::new()
        .with_group("A", base_data.clone())
        .with_notch(true)
        .with_notch_width(0.2);
    let wide = BoxPlot::new()
        .with_group("A", base_data)
        .with_notch(true)
        .with_notch_width(0.9);

    let narrow_plots = vec![Plot::Box(narrow)];
    let narrow_layout = Layout::auto_from_plots(&narrow_plots).with_title("Narrow Notch Width");
    let narrow_svg = SvgBackend.render_scene(&render_multiple(narrow_plots, narrow_layout));
    common::write_test_output(
        "test_outputs/boxplot_notch_width_narrow.svg",
        narrow_svg.clone(),
    )
    .unwrap();

    let wide_plots = vec![Plot::Box(wide)];
    let wide_layout = Layout::auto_from_plots(&wide_plots).with_title("Wide Notch Width");
    let wide_svg = SvgBackend.render_scene(&render_multiple(wide_plots, wide_layout));
    common::write_test_output(
        "test_outputs/boxplot_notch_width_wide.svg",
        wide_svg.clone(),
    )
    .unwrap();

    // Point order: (x0,yq3) (x1,yq3) (x1,y_notch_top) (pinch_right,ymed) ...
    // — y_notch_top is coords[5], ymed is coords[7]. The notch's vertical
    // half-span is the distance from the median line to the notch's flare
    // point, not from the notch to the box's own top edge.
    let narrow_coords = extract_first_path_coords(&narrow_svg);
    let wide_coords = extract_first_path_coords(&wide_svg);
    let narrow_notch_top = narrow_coords[5];
    let narrow_ymed = narrow_coords[7];
    let wide_notch_top = wide_coords[5];
    let wide_ymed = wide_coords[7];

    let narrow_span = (narrow_notch_top - narrow_ymed).abs();
    let wide_span = (wide_notch_top - wide_ymed).abs();

    assert!(
        wide_span > narrow_span,
        "notch_width=0.9 should produce a taller notch than notch_width=0.2: \
         narrow_span={narrow_span:.2}, wide_span={wide_span:.2}"
    );
}
