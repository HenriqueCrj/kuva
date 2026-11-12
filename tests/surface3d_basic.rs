mod common;
use kuva::backend::svg::SvgBackend;
use kuva::plot::heatmap::ColorMap;
use kuva::plot::scatter3d::Scatter3DPlot;
use kuva::plot::surface3d::Surface3DPlot;
use kuva::render::layout::Layout;
use kuva::render::plots::Plot;
use kuva::render::render::render_multiple;

fn paraboloid_grid(n: usize) -> Vec<Vec<f64>> {
    (0..n)
        .map(|i| {
            (0..n)
                .map(|j| {
                    let x = (i as f64 - n as f64 / 2.0) / (n as f64 / 4.0);
                    let y = (j as f64 - n as f64 / 2.0) / (n as f64 / 4.0);
                    x * x + y * y
                })
                .collect()
        })
        .collect()
}

#[test]
fn test_surface3d_basic() {
    let surface = Surface3DPlot::new(paraboloid_grid(10)).with_color("steelblue");

    let plots = vec![Plot::Surface3D(surface)];
    let layout = Layout::auto_from_plots(&plots).with_title("Surface3D Basic");
    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/surface3d_basic.svg", svg.clone()).unwrap();
    assert!(svg.contains("<svg"));
    assert!(
        svg.contains("<path"),
        "should contain path elements for surface faces"
    );
}

#[test]
fn test_surface3d_colormap() {
    let surface = Surface3DPlot::new(paraboloid_grid(15)).with_z_colormap(ColorMap::Viridis);

    let plots = vec![Plot::Surface3D(surface)];
    let layout = Layout::auto_from_plots(&plots);
    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/surface3d_colormap.svg", svg.clone()).unwrap();
    assert!(svg.contains("<svg"));
    let path_count = svg.matches("<path").count();
    // Should have many paths: back panes + grid + surface faces
    assert!(
        path_count > 100,
        "colormap surface should have many paths, got {path_count}"
    );
}

#[test]
fn test_surface3d_no_wireframe() {
    let surface = Surface3DPlot::new(paraboloid_grid(10))
        .with_no_wireframe()
        .with_z_colormap(ColorMap::Inferno);

    let plots = vec![Plot::Surface3D(surface)];
    let layout = Layout::auto_from_plots(&plots);
    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/surface3d_no_wireframe.svg", svg.clone()).unwrap();
    assert!(svg.contains("<svg"));
    // Wireframe off: stroke should be "none" on surface faces
    assert!(svg.contains("stroke=\"none\""));
}

#[test]
fn test_surface3d_alpha() {
    let surface = Surface3DPlot::new(paraboloid_grid(8))
        .with_alpha(0.7)
        .with_color("coral");

    let plots = vec![Plot::Surface3D(surface)];
    let layout = Layout::auto_from_plots(&plots);
    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/surface3d_alpha.svg", svg.clone()).unwrap();
    assert!(svg.contains("<svg"));
    assert!(
        svg.contains("opacity"),
        "should contain opacity attribute for alpha"
    );
}

#[test]
fn test_surface3d_empty() {
    let surface = Surface3DPlot::new(vec![]);
    let plots = vec![Plot::Surface3D(surface)];
    let layout = Layout::auto_from_plots(&plots);
    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/surface3d_empty.svg", svg.clone()).unwrap();
    assert!(svg.contains("<svg"));
}

#[test]
fn test_surface3d_axis_labels() {
    let surface = Surface3DPlot::new(paraboloid_grid(8))
        .with_x_label("X Axis")
        .with_y_label("Y Axis")
        .with_z_label("Z Axis");

    let plots = vec![Plot::Surface3D(surface)];
    let layout = Layout::auto_from_plots(&plots);
    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/surface3d_labels.svg", svg.clone()).unwrap();
    assert!(svg.contains("X Axis"));
    assert!(svg.contains("Y Axis"));
    assert!(svg.contains("Z Axis"));
}

#[test]
fn test_surface3d_explicit_coords() {
    let xs: Vec<f64> = (-5..=5).map(|i| i as f64).collect();
    let ys: Vec<f64> = (-5..=5).map(|i| i as f64).collect();
    let z_data: Vec<Vec<f64>> = ys
        .iter()
        .map(|&y| xs.iter().map(|&x| (x * x + y * y).sqrt().sin()).collect())
        .collect();

    let surface = Surface3DPlot::new(z_data)
        .with_x_coords(xs)
        .with_y_coords(ys)
        .with_z_colormap(ColorMap::Viridis);

    let plots = vec![Plot::Surface3D(surface)];
    let layout = Layout::auto_from_plots(&plots);
    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/surface3d_coords.svg", svg.clone()).unwrap();
    assert!(svg.contains("<svg"));
}

#[test]
fn test_surface3d_custom_view() {
    let surface = Surface3DPlot::new(paraboloid_grid(10))
        .with_azimuth(-120.0)
        .with_elevation(20.0)
        .with_z_colormap(ColorMap::Viridis);

    let plots = vec![Plot::Surface3D(surface)];
    let layout = Layout::auto_from_plots(&plots);
    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/surface3d_custom_view.svg", svg.clone()).unwrap();
    assert!(svg.contains("<svg"));
}

#[test]
fn test_surface3d_with_data_fn() {
    let surface = Surface3DPlot::new(vec![])
        .with_data_fn(
            |x, y| (x * x + y * y).sqrt().sin(),
            -3.0..=3.0,
            -3.0..=3.0,
            30,
            30,
        )
        .with_z_colormap(ColorMap::Viridis);

    assert_eq!(surface.nrows(), 30);
    assert_eq!(surface.ncols(), 30);

    let plots = vec![Plot::Surface3D(surface)];
    let layout = Layout::auto_from_plots(&plots).with_title("with_data_fn");
    let scene = render_multiple(plots, layout);
    let svg = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/surface3d_data_fn.svg", svg.clone()).unwrap();
    assert!(svg.contains("<svg"));
    // 30x30 grid = 29x29 = 841 faces + back panes → lots of paths
    let path_count = svg.matches("<path").count();
    assert!(
        path_count > 800,
        "high-res surface should have many paths, got {path_count}"
    );
}

#[test]
fn test_surface3d_with_data_fn_low_vs_high_res() {
    // Low res
    let low =
        Surface3DPlot::new(vec![]).with_data_fn(|x, y| x * x + y * y, -2.0..=2.0, -2.0..=2.0, 5, 5);
    assert_eq!(low.nrows(), 5);
    assert_eq!(low.ncols(), 5);

    let plots = vec![Plot::Surface3D(low)];
    let layout = Layout::auto_from_plots(&plots);
    let scene = render_multiple(plots, layout);
    let svg_low = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/surface3d_low_res.svg", svg_low.clone()).unwrap();

    // High res
    let high = Surface3DPlot::new(vec![]).with_data_fn(
        |x, y| x * x + y * y,
        -2.0..=2.0,
        -2.0..=2.0,
        40,
        40,
    );
    assert_eq!(high.nrows(), 40);
    assert_eq!(high.ncols(), 40);

    let plots = vec![Plot::Surface3D(high)];
    let layout = Layout::auto_from_plots(&plots);
    let scene = render_multiple(plots, layout);
    let svg_high = SvgBackend.render_scene(&scene);
    common::write_test_output("test_outputs/surface3d_high_res.svg", svg_high.clone()).unwrap();

    // High res should produce significantly more SVG content
    assert!(
        svg_high.len() > svg_low.len() * 5,
        "high-res ({} bytes) should be much larger than low-res ({} bytes)",
        svg_high.len(),
        svg_low.len()
    );
}

// Mixing a Scatter3D and a Surface3D in one panel used to have each draw its own
// wireframe box independently (same underlying bug as combining multiple
// Scatter3D instances — see scatter3d_basic.rs). The back-pane fill count is a
// range-independent invariant for detecting a doubled box: which faces are
// "back-facing" (and so get a translucent fill) depends only on the view
// angles, never on data range, so it must be identical whether one or several
// 3D plots share the box.
#[test]
fn test_scatter3d_and_surface3d_share_one_box() {
    let scatter = Scatter3DPlot::new().with_data(vec![(0.0, 0.0, 20.0), (1.0, 1.0, 25.0)]);
    let surface = Surface3DPlot::new(paraboloid_grid(10));

    let mixed_plots = vec![Plot::Scatter3D(scatter), Plot::Surface3D(surface.clone())];
    let mixed_layout = Layout::auto_from_plots(&mixed_plots);
    let mixed_svg = SvgBackend.render_scene(&render_multiple(mixed_plots, mixed_layout));
    common::write_test_output(
        "test_outputs/surface3d_mixed_with_scatter3d.svg",
        mixed_svg.clone(),
    )
    .unwrap();

    let solo_plots = vec![Plot::Surface3D(surface)];
    let solo_layout = Layout::auto_from_plots(&solo_plots);
    let solo_svg = SvgBackend.render_scene(&render_multiple(solo_plots, solo_layout));

    let pane_fill_count = |svg: &str| svg.matches("opacity=\"0.15\"").count();
    let mixed_panes = pane_fill_count(&mixed_svg);
    let solo_panes = pane_fill_count(&solo_svg);
    assert!(mixed_panes > 0, "expected at least one back-pane fill");
    assert_eq!(
        mixed_panes, solo_panes,
        "a Scatter3D combined with a Surface3D should still draw exactly one box \
         (same back-pane-fill count, {solo_panes}, as the Surface3D alone) — got \
         {mixed_panes}, which would indicate the box was drawn twice"
    );
}
