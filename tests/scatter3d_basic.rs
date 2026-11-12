mod common;
use kuva::backend::svg::SvgBackend;
use kuva::plot::heatmap::ColorMap;
use kuva::plot::scatter::MarkerShape;
use kuva::plot::scatter3d::Scatter3DPlot;
use kuva::render::layout::Layout;
use kuva::render::plots::Plot;
use kuva::render::render::render_multiple;

#[test]
fn test_scatter3d_basic() {
    let data = vec![
        (1.0, 2.0, 3.0),
        (4.0, 5.0, 6.0),
        (7.0, 8.0, 9.0),
        (2.0, 6.0, 1.0),
        (5.0, 3.0, 7.0),
    ];

    let plot = Scatter3DPlot::new().with_data(data).with_color("steelblue");

    let plots = vec![Plot::Scatter3D(plot)];
    let layout = Layout::auto_from_plots(&plots).with_title("3D Scatter Basic");

    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/scatter3d_basic.svg", svg.clone()).unwrap();
    assert!(svg.contains("<svg"));
    // Should contain circle markers
    assert!(svg.contains("<circle"), "SVG should contain circle markers");
}

#[test]
fn test_scatter3d_wireframe() {
    let data = vec![(0.0, 0.0, 0.0), (1.0, 1.0, 1.0)];
    let plot = Scatter3DPlot::new().with_data(data);

    let plots = vec![Plot::Scatter3D(plot)];
    let layout = Layout::auto_from_plots(&plots);
    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/scatter3d_wireframe.svg", svg.clone()).unwrap();
    assert!(svg.contains("<svg"));
    // Wireframe box produces line elements
    assert!(
        svg.contains("<line"),
        "SVG should contain line elements for wireframe"
    );
}

#[test]
fn test_scatter3d_custom_view() {
    let data = vec![(1.0, 2.0, 3.0), (4.0, 5.0, 6.0)];

    let plot = Scatter3DPlot::new()
        .with_data(data)
        .with_azimuth(-30.0)
        .with_elevation(45.0);

    let plots = vec![Plot::Scatter3D(plot)];
    let layout = Layout::auto_from_plots(&plots);
    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/scatter3d_custom_view.svg", svg.clone()).unwrap();
    assert!(svg.contains("<svg"));
    assert!(svg.contains("<circle"));
}

#[test]
fn test_scatter3d_z_colormap() {
    let data: Vec<(f64, f64, f64)> = (0..20)
        .map(|i| {
            let t = i as f64 / 19.0;
            (t * 10.0, t.sin() * 5.0, t * 8.0)
        })
        .collect();

    let plot = Scatter3DPlot::new()
        .with_data(data)
        .with_z_colormap(ColorMap::Viridis);

    let plots = vec![Plot::Scatter3D(plot)];
    let layout = Layout::auto_from_plots(&plots);
    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/scatter3d_colormap.svg", svg.clone()).unwrap();
    assert!(svg.contains("<svg"));
    // Z-colormap produces varied fill colors from viridis (rendered as hex)
    // With 20 points at different z values, there should be multiple distinct fill colors
    let circle_count = svg.matches("<circle").count();
    assert!(
        circle_count >= 15,
        "SVG should contain many circle markers, got {circle_count}"
    );
}

#[test]
fn test_scatter3d_depth_shade() {
    let data = vec![(0.0, 0.0, 0.0), (5.0, 5.0, 5.0), (10.0, 10.0, 10.0)];

    let plot = Scatter3DPlot::new().with_data(data).with_depth_shade();

    let plots = vec![Plot::Scatter3D(plot)];
    let layout = Layout::auto_from_plots(&plots);
    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/scatter3d_depth_shade.svg", svg.clone()).unwrap();
    assert!(svg.contains("<svg"));
    // Depth shading produces opacity attributes
    assert!(
        svg.contains("fill-opacity"),
        "SVG should contain fill-opacity for depth shading"
    );
}

#[test]
fn test_scatter3d_legend() {
    let plot = Scatter3DPlot::new()
        .with_data(vec![(1.0, 2.0, 3.0), (4.0, 5.0, 6.0)])
        .with_legend("Group A");

    let plots = vec![Plot::Scatter3D(plot)];
    let layout = Layout::auto_from_plots(&plots);
    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/scatter3d_legend.svg", svg.clone()).unwrap();
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Group A"), "SVG should contain legend label");
}

#[test]
fn test_scatter3d_empty() {
    let plot = Scatter3DPlot::new();

    let plots = vec![Plot::Scatter3D(plot)];
    let layout = Layout::auto_from_plots(&plots);
    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/scatter3d_empty.svg", svg.clone()).unwrap();
    assert!(svg.contains("<svg"));
    // Should not crash, just produce an empty-ish SVG
}

#[test]
fn test_scatter3d_axis_labels() {
    let data = vec![(1.0, 2.0, 3.0), (4.0, 5.0, 6.0)];
    let plot = Scatter3DPlot::new()
        .with_data(data)
        .with_x_label("X Axis")
        .with_y_label("Y Axis")
        .with_z_label("Z Axis");

    let plots = vec![Plot::Scatter3D(plot)];
    let layout = Layout::auto_from_plots(&plots);
    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/scatter3d_labels.svg", svg.clone()).unwrap();
    assert!(svg.contains("<svg"));
    assert!(svg.contains("X Axis"), "SVG should contain X axis label");
    assert!(svg.contains("Y Axis"), "SVG should contain Y axis label");
    assert!(svg.contains("Z Axis"), "SVG should contain Z axis label");
}

#[test]
fn test_scatter3d_marker_shapes() {
    let data = vec![(1.0, 2.0, 3.0), (4.0, 5.0, 6.0)];
    let plot = Scatter3DPlot::new()
        .with_data(data)
        .with_marker(MarkerShape::Square);

    let plots = vec![Plot::Scatter3D(plot)];
    let layout = Layout::auto_from_plots(&plots);
    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/scatter3d_squares.svg", svg.clone()).unwrap();
    assert!(svg.contains("<svg"));
    // Square markers produce rect elements
    assert!(
        svg.contains("<rect"),
        "SVG should contain rect elements for square markers"
    );
}

#[test]
fn test_scatter3d_no_grid_no_box() {
    let data = vec![(1.0, 2.0, 3.0), (4.0, 5.0, 6.0)];
    let plot = Scatter3DPlot::new()
        .with_data(data)
        .with_no_grid()
        .with_no_box();

    let plots = vec![Plot::Scatter3D(plot)];
    let layout = Layout::auto_from_plots(&plots);
    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/scatter3d_no_grid_box.svg", svg.clone()).unwrap();
    assert!(svg.contains("<svg"));
    assert!(svg.contains("<circle"), "should still have data points");
}

#[test]
fn test_scatter3d_auto_z_axis() {
    let data = vec![(0.0, 0.0, 0.0), (10.0, 10.0, 10.0)];

    // Default view (azimuth=-60°): auto should place Z on the right.
    // Mirrored view (azimuth=+60°): auto should flip Z to the left.
    // Both should render without panicking and produce valid SVG.
    let default_plot = Scatter3DPlot::new()
        .with_data(data.clone())
        .with_x_label("X")
        .with_y_label("Y")
        .with_z_label("Z");

    let plots = vec![Plot::Scatter3D(default_plot)];
    let layout = Layout::auto_from_plots(&plots);
    let svg_default = SvgBackend.render_scene(&render_multiple(plots, layout));
    common::write_test_output(
        "test_outputs/scatter3d_auto_z_default.svg",
        svg_default.clone(),
    )
    .unwrap();
    assert!(svg_default.contains("Z"), "Z axis label should be present");

    let mirrored_plot = Scatter3DPlot::new()
        .with_data(data.clone())
        .with_azimuth(60.0)
        .with_x_label("X")
        .with_y_label("Y")
        .with_z_label("Z");

    let plots = vec![Plot::Scatter3D(mirrored_plot)];
    let layout = Layout::auto_from_plots(&plots);
    let svg_mirrored = SvgBackend.render_scene(&render_multiple(plots, layout));
    common::write_test_output(
        "test_outputs/scatter3d_auto_z_mirrored.svg",
        svg_mirrored.clone(),
    )
    .unwrap();
    assert!(svg_mirrored.contains("Z"), "Z axis label should be present");

    // Explicit override: force left even at default azimuth
    let forced_left = Scatter3DPlot::new()
        .with_data(data)
        .with_z_axis_right(false)
        .with_z_label("Z");

    let plots = vec![Plot::Scatter3D(forced_left)];
    let layout = Layout::auto_from_plots(&plots);
    let svg_left = SvgBackend.render_scene(&render_multiple(plots, layout));
    common::write_test_output("test_outputs/scatter3d_z_axis_left.svg", svg_left.clone()).unwrap();
    assert!(svg_left.contains("Z"));
}

// Collects every `<circle cx="..." cy="...">` center from an SVG string.
fn circle_centers(svg: &str) -> Vec<(f64, f64)> {
    let mut centers = Vec::new();
    for chunk in svg.split("<circle") {
        let cx = chunk
            .split("cx=\"")
            .nth(1)
            .and_then(|s| s.split('"').next())
            .and_then(|s| s.parse::<f64>().ok());
        let cy = chunk
            .split("cy=\"")
            .nth(1)
            .and_then(|s| s.split('"').next())
            .and_then(|s| s.parse::<f64>().ok());
        if let (Some(cx), Some(cy)) = (cx, cy) {
            centers.push((cx, cy));
        }
    }
    centers
}

// Multiple `Scatter3D` instances in one figure used to each normalize independently
// to their own min/max, so two 2-point series with very different values always
// projected to the *same* two screen corners regardless of what their values
// actually were. With a shared coordinate box, two disjoint-range series should
// land at 4 distinct screen positions, not collapse pairwise onto 2.
#[test]
fn test_scatter3d_multiple_instances_share_one_coordinate_box() {
    let plot_a = Scatter3DPlot::new()
        .with_data(vec![(1.0, 2.0, 3.0), (2.0, 3.0, 4.0)])
        .with_color("steelblue");
    let plot_b = Scatter3DPlot::new()
        .with_data(vec![(40.0, 50.0, 10.0), (50.0, 60.0, 20.0)])
        .with_color("tomato");

    let plots = vec![Plot::Scatter3D(plot_a), Plot::Scatter3D(plot_b)];
    let layout = Layout::auto_from_plots(&plots).with_title("Shared 3D coordinate box");
    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output(
        "test_outputs/scatter3d_shared_coordinate_box.svg",
        svg.clone(),
    )
    .unwrap();

    let centers = circle_centers(&svg);
    assert_eq!(
        centers.len(),
        4,
        "expected 4 data-point circles (2 per instance), got {}",
        centers.len()
    );
    let unique: std::collections::HashSet<(i64, i64)> = centers
        .iter()
        .map(|&(x, y)| ((x * 100.0).round() as i64, (y * 100.0).round() as i64))
        .collect();
    assert_eq!(
        unique.len(),
        4,
        "each of the 4 points should project to a distinct screen position when \
         instances share one coordinate box; got {} distinct positions from {:?}",
        unique.len(),
        centers
    );
}

// Two Scatter3D instances that would draw different wireframe boxes if each
// computed its own (e.g. differing `--no-grid` config) must share exactly one
// drawn box, sized from the *first* instance's config — not double-draw.
#[test]
fn test_scatter3d_multiple_instances_draw_one_shared_box() {
    let plot_a = Scatter3DPlot::new().with_data(vec![(0.0, 0.0, 0.0), (1.0, 1.0, 1.0)]);
    let plot_b = Scatter3DPlot::new().with_data(vec![(5.0, 5.0, 5.0), (6.0, 6.0, 6.0)]);

    let plots = vec![Plot::Scatter3D(plot_a), Plot::Scatter3D(plot_b)];
    let layout = Layout::auto_from_plots(&plots);
    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    common::write_test_output("test_outputs/scatter3d_one_shared_box.svg", svg.clone()).unwrap();

    // 12 box edges + grid lines drawn once; if the box were drawn twice this would
    // be roughly double. Compared against a *single* instance spanning the exact
    // same merged range (0..6) rather than either individual instance's own
    // narrower range — tick count (and so line count) is range-dependent
    // (`generate_ticks` nice-rounds differently for a 0..1 span than a 0..6
    // span), so the baseline must use the same combined range to be a fair
    // same-box-once comparison.
    let equivalent_range_plot =
        Scatter3DPlot::new().with_data(vec![(0.0, 0.0, 0.0), (6.0, 6.0, 6.0)]);
    let single_plots = vec![Plot::Scatter3D(equivalent_range_plot)];
    let single_layout = Layout::auto_from_plots(&single_plots);
    let single_svg = SvgBackend.render_scene(&render_multiple(single_plots, single_layout));

    let multi_lines = svg.matches("<line").count();
    let single_lines = single_svg.matches("<line").count();
    assert_eq!(
        multi_lines, single_lines,
        "two Scatter3D instances should draw exactly one wireframe box (same line \
         count as a single instance spanning the same merged range), got {multi_lines} \
         lines vs {single_lines} for the equivalent-range single instance"
    );
}
