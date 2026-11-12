//! `Layout::with_subtitle` renders a second line centred under the title at a smaller,
//! muted size, and the title block reserves the extra height.

mod common;

use kuva::backend::svg::SvgBackend;
use kuva::plot::scatter::ScatterPlot;
use kuva::render::layout::Layout;
use kuva::render::render::render_scatter;

fn scatter_svg(layout: Layout) -> String {
    let plot = ScatterPlot::new().with_data(vec![(1.0, 5.0), (4.5, 3.5), (5.0, 8.7)]);
    SvgBackend.render_scene(&render_scatter(&plot, layout).with_background(Some("white")))
}

/// Read a numeric attribute from the root `<svg>` element.
fn svg_root_attr(svg: &str, name: &str) -> f64 {
    let head = &svg[..svg.find('>').unwrap()];
    let key = format!("{name}=\"");
    let s = head.find(&key).unwrap() + key.len();
    let e = head[s..].find('"').unwrap() + s;
    head[s..e].parse().unwrap()
}

/// Return the full `<text …>content</text>` tag whose content is exactly `content`.
fn text_tag(svg: &str, content: &str) -> String {
    let needle = format!(">{content}</text>");
    let close = svg
        .find(&needle)
        .unwrap_or_else(|| panic!("no <text> {content:?}"));
    let open = svg[..close].rfind("<text ").unwrap();
    svg[open..close + needle.len()].to_string()
}

fn base_layout() -> Layout {
    Layout::new((0.0, 10.0), (0.0, 10.0)).with_title("Main Title")
}

#[test]
fn subtitle_text_is_rendered() {
    let svg = scatter_svg(base_layout().with_subtitle("n = 1,234 cells"));
    common::write_test_output("test_outputs/subtitle_basic.svg", &svg).unwrap();
    assert!(
        svg.contains(">n = 1,234 cells</text>"),
        "subtitle text should be drawn"
    );
}

#[test]
fn subtitle_is_smaller_and_muted() {
    let svg = scatter_svg(base_layout().with_subtitle("a subtitle"));
    let title = text_tag(&svg, "Main Title");
    let subtitle = text_tag(&svg, "a subtitle");

    // Default title size is 18 → subtitle is round(18 * 0.7) = 13.
    assert!(
        title.contains("font-size=\"18\""),
        "title at default 18: {title}"
    );
    assert!(
        subtitle.contains("font-size=\"13\""),
        "subtitle at 0.7x = 13: {subtitle}"
    );
    // Subtitle is muted; the title inherits the default fill (no per-element fill).
    assert!(
        subtitle.contains("fill=\"#666666\""),
        "subtitle should be muted: {subtitle}"
    );
    assert!(
        !title.contains("fill="),
        "title should not set a per-element fill: {title}"
    );
    // Both are centre-anchored on the same x.
    assert!(subtitle.contains("text-anchor=\"middle\""));
}

#[test]
fn subtitle_reserves_extra_top_margin() {
    let without = scatter_svg(base_layout());
    let with = scatter_svg(base_layout().with_subtitle("a subtitle"));
    assert!(
        svg_root_attr(&with, "height") > svg_root_attr(&without, "height"),
        "subtitle must add top-margin height ({} !> {})",
        svg_root_attr(&with, "height"),
        svg_root_attr(&without, "height"),
    );
}

#[test]
fn subtitle_sits_below_title() {
    let svg = scatter_svg(base_layout().with_subtitle("a subtitle"));
    let y_of = |tag: &str| -> f64 {
        let s = tag.find("y=\"").unwrap() + 3;
        let e = tag[s..].find('"').unwrap() + s;
        tag[s..e].parse().unwrap()
    };
    assert!(
        y_of(&text_tag(&svg, "a subtitle")) > y_of(&text_tag(&svg, "Main Title")),
        "subtitle baseline should be below the title baseline"
    );
}

#[test]
fn subtitle_size_can_be_set_explicitly() {
    let svg = scatter_svg(base_layout().with_subtitle("sized").with_subtitle_size(24));
    let subtitle = text_tag(&svg, "sized");
    assert!(
        subtitle.contains("font-size=\"24\""),
        "explicit subtitle size overrides the 0.7x default: {subtitle}"
    );
}

#[test]
fn subtitle_wrap_is_independent_of_title_wrap() {
    // title_wrap set but subtitle_wrap unset: the subtitle must NOT inherit it.
    let svg = scatter_svg(base_layout().with_subtitle("alpha beta").with_title_wrap(5));
    assert!(
        svg.contains(">alpha beta</text>"),
        "subtitle should stay on one line when only title_wrap is set"
    );
    // subtitle_wrap set: the subtitle wraps at its own width, into two <text> lines.
    let svg = scatter_svg(
        base_layout()
            .with_subtitle("alpha beta")
            .with_subtitle_wrap(5),
    );
    assert!(
        svg.contains(">alpha</text>"),
        "subtitle should wrap on its own width"
    );
    assert!(
        svg.contains(">beta</text>"),
        "subtitle should wrap on its own width"
    );
    assert!(
        !svg.contains(">alpha beta</text>"),
        "wrapped subtitle should not also appear as one line"
    );
}

#[test]
fn subtitle_colour_is_derived_from_the_theme() {
    // Dark theme: light text (#e0e0e0) on dark bg (#1e1e1e). The muted subtitle is the
    // title colour blended 0.4 toward the background = #929292 (224·0.6 + 30·0.4 = 146),
    // i.e. derived from the theme, not the light-theme grey.
    let plot = ScatterPlot::new().with_data(vec![(1.0, 5.0), (4.5, 3.5), (5.0, 8.7)]);
    let layout = Layout::new((0.0, 10.0), (0.0, 10.0))
        .with_title("Main Title")
        .with_subtitle("muted")
        .with_theme(kuva::render::theme::Theme::dark());
    let svg = SvgBackend.render_scene(&render_scatter(&plot, layout));
    let subtitle = text_tag(&svg, "muted");
    assert!(
        subtitle.contains("fill=\"#929292\""),
        "dark-theme subtitle should use the theme-derived muted colour, not #666666: {subtitle}"
    );
}

#[test]
fn subtitle_renders_without_a_title() {
    let svg = scatter_svg(Layout::new((0.0, 10.0), (0.0, 10.0)).with_subtitle("solo"));
    assert!(
        svg.contains(">solo</text>"),
        "subtitle should render even when no title is set"
    );
}

#[test]
fn global_wrap_also_wraps_the_subtitle() {
    // `with_wrap` seeds the subtitle wrap when it isn't set explicitly.
    let svg = scatter_svg(base_layout().with_subtitle("alpha beta").with_wrap(5));
    assert!(
        svg.contains(">alpha</text>"),
        "global wrap should reach the subtitle"
    );
    assert!(
        svg.contains(">beta</text>"),
        "global wrap should reach the subtitle"
    );
    assert!(!svg.contains(">alpha beta</text>"));
}

#[test]
fn subtitle_falls_back_to_title_colour_when_theme_is_unparseable() {
    // A theme whose colours use CSS kuva can't resolve to RGB (`hsl()`): the subtitle
    // can't be muted, so it reuses the title's own colour un-muted rather than
    // fabricating the light-theme grey.
    let mut theme = kuva::render::theme::Theme::dark();
    theme.text_color = "hsl(0, 0%, 88%)".to_string();
    theme.background = "hsl(0, 0%, 10%)".to_string();
    let plot = ScatterPlot::new().with_data(vec![(1.0, 5.0), (4.5, 3.5), (5.0, 8.7)]);
    let layout = Layout::new((0.0, 10.0), (0.0, 10.0))
        .with_title("Main Title")
        .with_subtitle("muted")
        .with_theme(theme);
    let svg = SvgBackend.render_scene(&render_scatter(&plot, layout));
    let subtitle = text_tag(&svg, "muted");
    assert!(
        subtitle.contains("fill=\"hsl(0, 0%, 88%)\""),
        "unparseable theme: subtitle should reuse the title colour un-muted: {subtitle}"
    );
    assert!(
        !subtitle.contains("#666666"),
        "should not fabricate the light-theme grey: {subtitle}"
    );
}

#[test]
fn figure_panel_keeps_its_subtitle() {
    // Multi-panel figures clone each cell's layout via `clone_layout`; the
    // per-panel subtitle must survive that copy (it did not initially).
    use kuva::render::figure::Figure;
    use kuva::render::plots::Plot;
    let plots = vec![Plot::Scatter(
        ScatterPlot::new().with_data(vec![(1.0, 2.0), (3.0, 4.0)]),
    )];
    let panel = Layout::new((0.0, 5.0), (0.0, 5.0))
        .with_title("Panel")
        .with_subtitle("panel subtitle");
    let figure = Figure::new(1, 1)
        .with_plots(vec![plots])
        .with_layouts(vec![panel]);
    let svg = SvgBackend.render_scene(&figure.render());
    assert!(
        svg.contains(">panel subtitle</text>"),
        "a per-panel subtitle should survive clone_layout into the figure"
    );
}
