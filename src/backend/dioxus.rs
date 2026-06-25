//! Native Dioxus (RSX) backend: `Scene` -> `dioxus::Element`.
//!
//! A new *sibling* serializer over `&Scene`, exactly like [`crate::backend::svg`]
//! and [`crate::backend::terminal`] — it implements no shared trait (there isn't
//! one) and touches no plot/layout/projection code. Where `SvgBackend` writes an
//! SVG **string**, `KuvaDioxus` builds a typed RSX node tree so a Dioxus 0.7 app
//! can render a kuva plot through the real VDOM (fine-grained diffing, no
//! `dangerous_inner_html`, signals/i18n reachable) instead of injecting an opaque
//! SVG blob.
//!
//! The two backends consume the **same** `Scene`, so their output must stay
//! visually identical. To make that true the float/color/text formatting here
//! mirrors `svg.rs` (see [`fmt_float`], the [`Color`] reuse, and [`text_content`]);
//! the unit tests below pin that parity against the live `SvgBackend` output
//! rather than against hand-written strings. The one expected divergence is the
//! escaping of special characters in text/attribute content: `svg.rs` writes
//! named XML entities (`&apos;`) while RSX auto-escapes (and may use numeric
//! entities like `&#39;`). The rendered glyphs are identical, so this is a
//! visual no-op — the output is *visually* identical, not byte-identical for text
//! containing `& < > " '`.
//!
//! ## Scope
//!
//! Maps every leaf primitive — `Circle`, `Rect`, `Text`, `Line`, `Path`,
//! `PolyLine`, the multi-span `RichText`, and the struct-of-arrays
//! `CircleBatch`/`RectBatch` — plus the `<svg>` root's width/height/font/fill.
//! The one piece of genuinely novel work is structural: the flat
//! `GroupStart/End` + `ClipStart/End` marker stream is folded into a nested
//! node-tree (see [`fold_scene`] / [`Node`]) and emitted as nested `<g>`
//! wrappers. `Scene.defs` (gradients/markers and the `<clipPath>` *definitions*
//! the fold's clip wrappers reference) goes into a single `<defs>`
//! `dangerous_inner_html` island (see [`defs_inner_html`]) — the **only**
//! `dangerous_inner_html` use in this backend.
//!
//! ## Dropping `Scene.scripts`
//!
//! `Scene.scripts` (kuva's baked interactivity JS) is **not** emitted. The
//! native, signal-driven interactivity below (hover tooltips, legend toggle,
//! coordinate readout) replaces it, and `<script>` injected via
//! `dangerous_inner_html` never auto-executes in the VDOM (so it was already
//! inert) — it is the very CSP/trust concern going native was meant to remove.
//! `Scene.scripts` has exactly one producer — `CalendarPlot` (`CALENDAR_TIP_JS`).
//! A `CalendarPlot` rendered through this backend therefore loses only that
//! already-inert JS; reach for the native interaction hooks instead.
//!
//! ## Responsive sizing
//!
//! Where `SvgBackend` bakes a fixed `width`/`height` into the root `<svg>`, this
//! backend instead emits a `viewBox` derived from `Scene.width/height` (see
//! [`view_box`]) and **no** `width`/`height` attributes, so the `<svg>` scales to
//! its container — the common responsive-SVG convention (`viewBox="0 0 W H"` plus
//! a CSS class that sets e.g. `width:100%`). A consumer attaches that CSS via
//! [`KuvaDioxus::with_class`] (or the [`KuvaPlot`] `class` prop).
//!
//! ## Theming
//!
//! kuva bakes a background fill and a text color into the `Scene` (white / black
//! by default, or a [`crate::render::theme::Theme`]'s colors), which `SvgBackend`
//! emits verbatim — fine for a standalone file, but often wrong for a host
//! application that owns its own colors. So this backend's background rect and
//! root `fill` are the one place it *deliberately* diverges from `SvgBackend`: by
//! default it stays at parity (emits `Scene.background_color` / `Scene.text_color`),
//! but a consumer can remap them to its own tokens or defer to CSS via
//! [`Background`] / [`TextColor`] (the [`KuvaDioxus::with_background`] /
//! [`KuvaDioxus::with_text_color`] builders and the matching [`KuvaPlot`] props).
//! The full-bleed background rect is emitted first under `<svg>` so it paints
//! behind every element (mirroring `svg.rs`'s ordering).
//!
//! ### Why fold into an intermediate `Node` tree (not straight to `Element`)
//!
//! A Dioxus `Element` is opaque — tests can't introspect its structure. The
//! flat→nested fold is the one piece of real logic in this backend and must be
//! tested on the degenerate cases (unbalanced markers). So [`fold_scene`] is a
//! pure function `&[Primitive] -> Vec<Node>` that the unit tests assert on
//! directly; [`render_node`] then turns each `Node` into RSX. The fold mirrors
//! `svg.rs`, which treats `GroupEnd` and `ClipEnd` interchangeably as "close the
//! current wrapper" (both just emit `</g>` and pop one nesting level).

use std::collections::HashSet;

use dioxus::prelude::*;

use crate::render::color::Color;
use crate::render::layout::Layout;
use crate::render::math;
use crate::render::plots::Plot;
use crate::render::render::{render_multiple, AxisMeta, Primitive, Scene, TextAnchor, TextSpan};

/// Mirror of `svg::write_float`: round to 2 decimal places, strip a whole-number
/// `.0` (`3`, not `3.0`), and `ryu`-format otherwise. Kept identical so the
/// Dioxus attribute values match the SVG backend's character-for-character —
/// the load-bearing invariant for the golden-diff (see module docs).
fn fmt_float(v: f64) -> String {
    let v = (v * 100.0).round() * 0.01;
    if v.fract() == 0.0 && v.abs() < 1e15 {
        (v as i64).to_string()
    } else {
        let mut rb = ryu::Buffer::new();
        rb.format(v).to_string()
    }
}

/// SVG `text-anchor` keyword for an anchor — same mapping as `svg.rs`.
fn anchor_str(anchor: &TextAnchor) -> &'static str {
    match anchor {
        TextAnchor::Start => "start",
        TextAnchor::Middle => "middle",
        TextAnchor::End => "end",
    }
}

/// Lower any `$...$` math regions in a `Text`/label to inline Unicode, exactly as
/// `svg.rs` does before emitting the text node. No-op for plain labels.
fn text_content(content: &str) -> String {
    if math::needs_rewrite(content) {
        math::to_unicode(content)
    } else {
        content.to_string()
    }
}

/// The `transform="rotate(angle,x,y)"` value a rotated `Text` carries, matching
/// `svg.rs`'s `rotate(<angle>,<x>,<y>)` (each component via [`fmt_float`]).
fn rotate_transform(angle: f64, x: f64, y: f64) -> String {
    format!(
        "rotate({},{},{})",
        fmt_float(angle),
        fmt_float(x),
        fmt_float(y)
    )
}

/// Build the `d` attribute for a `PolyLine`, byte-for-byte as `svg.rs` does:
/// `M` for the first point and `L` for the rest, each followed by a single space,
/// the `x`/`y` (each via [`fmt_float`]) space-separated, and a **trailing space**
/// after every point. The trailing space matters for a clean golden-diff.
fn polyline_path_d(points: &[(f64, f64)]) -> String {
    let mut d = String::new();
    for (i, &(x, y)) in points.iter().enumerate() {
        d.push(if i == 0 { 'M' } else { 'L' });
        d.push(' ');
        d.push_str(&fmt_float(x));
        d.push(' ');
        d.push_str(&fmt_float(y));
        d.push(' ');
    }
    d
}

/// Map one `RichText` span to an RSX node, mirroring `svg.rs`: an unstyled span
/// is bare text, a styled one (`bold`/`italic`/`underline`/`code`) is a `<tspan>`
/// carrying — in the same order the SVG backend writes them — `font-family`
/// (monospace, for `code`), `font-weight`, `font-style`, and `text-decoration`.
/// Span text is **not** math-lowered (the SVG backend escapes it verbatim, with
/// no `math::to_unicode` pass — unlike a plain `Text`).
fn render_span(span: &TextSpan) -> Element {
    let styled = span.bold || span.italic || span.underline || span.code;
    if styled {
        rsx! {
            tspan {
                font_family: if span.code { Some("DejaVu Sans Mono,monospace") } else { None },
                font_weight: if span.bold { Some("bold") } else { None },
                font_style: if span.italic { Some("italic") } else { None },
                text_decoration: if span.underline { Some("underline") } else { None },
                "{span.text}"
            }
        }
    } else {
        rsx! {
            "{span.text}"
        }
    }
}

/// How the chart's full-bleed background rect is emitted, overriding the
/// kuva-baked `Scene.background_color` (the theming hook).
///
/// `SvgBackend` always emits `Scene.background_color` (kuva's baked white, or a
/// [`crate::render::theme::Theme`]'s background) as a `<rect width="100%"
/// height="100%">`. A host application often wants its *own* background, so this
/// backend lets the consumer remap it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Background {
    /// Emit `Scene.background_color` as a full-bleed rect, exactly as `SvgBackend`
    /// does (golden-diff parity). The default. `None` background in the `Scene`
    /// (already optional via [`Scene::with_background`]) emits no rect either way.
    #[default]
    Scene,
    /// Emit no background rect — the host page / surrounding surface shows
    /// through. The typical choice when the host owns its panel backgrounds.
    None,
    /// Emit a full-bleed rect with this fill — typically a host theme token,
    /// e.g. `"var(--surface)"`.
    Token(String),
}

/// How the root `<svg>`'s `fill` (kuva's text color) is set, overriding the
/// kuva-baked `Scene.text_color` (the theming hook). SVG text/glyphs inherit
/// `fill`, so this drives all label color.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TextColor {
    /// Emit `Scene.text_color` as the root `fill`, exactly as `SvgBackend` does
    /// (golden-diff parity). The default. A `None` text color emits no `fill`.
    #[default]
    Scene,
    /// Emit no `fill` attribute, so text color is inherited from CSS — pair with a
    /// [`KuvaDioxus::with_class`] CSS class (`.chart { fill: var(--text); }`).
    Inherit,
    /// Set the root `fill` to this token, e.g. `"currentColor"` or `"var(--text)"`.
    Token(String),
}

/// Live hover-tooltip state, written by the interactive group handlers and read
/// by [`KuvaTooltip`]. `None` when nothing is hovered. `text` is the
/// hovered group's `GroupStart.title`; `x`/`y` are the cursor's **viewport**
/// (client) coordinates, so the overlay is positioned with `position: fixed`.
///
/// `Clone + PartialEq` so it can ride a `Signal<Option<Tooltip>>` and a
/// `ReadSignal` prop (which memoize by the inner value), the same shape the
/// `KuvaPlot` `data` prop relies on.
#[derive(Debug, Clone, PartialEq)]
pub struct Tooltip {
    pub text: String,
    pub x: f64,
    pub y: f64,
    /// Whether this tooltip is *pinned* (click-to-pin). A pinned tooltip
    /// is frozen: subsequent hover enter/move/leave on its group neither update nor
    /// clear it (see [`hover_locked`]); it stays until the group is clicked again
    /// (see [`pin_on_click`]). Hover-shown tooltips and the coordinate readout are
    /// always unpinned (`false`).
    pub pinned: bool,
}

/// A `Copy` snapshot of [`AxisMeta`]'s fields, so the coordinate-readout handlers
/// can capture the axis calibration by value into an `'static` closure without
/// cloning — `AxisMeta` is `!Copy`, and this backend makes no changes to kuva's
/// render types. Built once per render via [`Axes::from_meta`].
#[derive(Debug, Clone, Copy, PartialEq)]
struct Axes {
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    plot_left: f64,
    plot_top: f64,
    plot_right: f64,
    plot_bottom: f64,
    log_x: bool,
    log_y: bool,
}

impl Axes {
    fn from_meta(m: &AxisMeta) -> Self {
        Self {
            x_min: m.x_min,
            x_max: m.x_max,
            y_min: m.y_min,
            y_max: m.y_max,
            plot_left: m.plot_left,
            plot_top: m.plot_top,
            plot_right: m.plot_right,
            plot_bottom: m.plot_bottom,
            log_x: m.log_x,
            log_y: m.log_y,
        }
    }
}

/// Convert a plot-area pixel `(px, py)` (in the `Scene`/`viewBox` coordinate
/// space) back to data coordinates, mirroring the injected JS `px2data`
/// ([`crate::backend::interactive_js`]) exactly — including the log-axis
/// `10^…` mapping and the y-flip (screen y grows downward, data y upward). This is
/// the native, hooks-free replacement for that JS readout.
fn px_to_data(a: Axes, px: f64, py: f64) -> (f64, f64) {
    let fx = (px - a.plot_left) / (a.plot_right - a.plot_left);
    let fy = (py - a.plot_top) / (a.plot_bottom - a.plot_top);
    let x = if a.log_x {
        10f64.powf(a.x_min + fx * (a.x_max - a.x_min))
    } else {
        a.x_min + fx * (a.x_max - a.x_min)
    };
    let y = if a.log_y {
        10f64.powf(a.y_max - fy * (a.y_max - a.y_min))
    } else {
        a.y_max - fy * (a.y_max - a.y_min)
    };
    (x, y)
}

/// Format one data value for the coordinate readout, modelled on the JS `fmt`:
/// exponential (2 decimals) for very large/small magnitudes, otherwise 3 decimal
/// places with trailing zeros (and a trailing `.`) trimmed. Not a golden-diff
/// surface — the JS readout is *replaced*, not co-rendered — so the only
/// divergence from `Number.toExponential(2)` (Rust omits the `+` in a positive
/// exponent, e.g. `1.23e4` vs `1.23e+4`) is intentional and irrelevant.
fn fmt_readout(v: f64) -> String {
    if v.abs() >= 1e4 || (v != 0.0 && v.abs() < 0.01) {
        format!("{v:.2e}")
    } else {
        // parseFloat(v.toFixed(3)).toString(): 3dp, then strip trailing zeros/dot.
        let s = format!("{v:.3}");
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        trimmed.to_string()
    }
}

/// The coordinate readout string (`"x=…  y=…"`, two spaces, matching the JS) for a
/// plot-area pixel, or `None` when the pixel falls outside the plot rectangle —
/// mirroring the JS, which clears the readout there. Pure, so the
/// pixel→data→string path is unit-tested without a render harness (the rendered
/// `<svg>` + handlers are opaque).
fn readout_text(a: Axes, px: f64, py: f64) -> Option<String> {
    if px < a.plot_left || px > a.plot_right || py < a.plot_top || py > a.plot_bottom {
        return None;
    }
    let (x, y) = px_to_data(a, px, py);
    Some(format!("x={}  y={}", fmt_readout(x), fmt_readout(y)))
}

/// A floating tooltip overlay driven by a `Signal<Option<Tooltip>>` — the native
/// replacement for the SVG backend's `<title>` element + injected hover JS —
/// renders a small box at the cursor when a titled group is hovered,
/// nothing otherwise.
///
/// It only **reads** the signal (no `use_*` hook), so — like the rest of this
/// backend — it needs no dioxus `hooks` feature: the consumer owns the
/// `use_signal` and passes the signal in (directly, or via [`KuvaPlot`]'s
/// `tooltip` prop, which renders this overlay for you).
///
/// ## Styling
///
/// The emitted markup is intentionally unopinionated: the inline style carries
/// **only** what must be computed at render time or is load-bearing for the
/// overlay mechanism — the cursor position, a small cursor offset,
/// `pointer-events: none` (so it never steals the hover it reports), and
/// `z-index`. **All appearance** (background, color, padding, font, line
/// wrapping) is left to the `.kuva-tooltip` CSS class, so the host app owns the
/// look with no baked-in theme.
///
/// Consequently the overlay renders **nothing visible until you add a
/// `.kuva-tooltip` rule** — that is deliberate (no opinionated default leaks into
/// a general-purpose library). A reasonable starting point:
///
/// ```css
/// .kuva-tooltip {
///   background: #222; color: #fff; padding: 2px 6px;
///   border-radius: 4px; font-size: 12px; white-space: nowrap;
/// }
/// ```
#[component]
pub fn KuvaTooltip(tooltip: ReadSignal<Option<Tooltip>>) -> Element {
    match tooltip() {
        Some(tip) => {
            // Inline ONLY positioning + overlay mechanics — `position: fixed` +
            // client coordinates places it relative to the viewport (unaffected by
            // the chart's layout), a small offset keeps it off the cursor, and
            // `pointer-events: none` keeps it from stealing the hover it reports.
            // ALL appearance (incl. white-space) is left to `.kuva-tooltip` (see
            // the doc comment) so this backend bakes in no theme.
            let style = format!(
                "position: fixed; left: {}px; top: {}px; transform: translate(12px, 12px); \
                 pointer-events: none; z-index: 1000;",
                fmt_float(tip.x),
                fmt_float(tip.y),
            );
            rsx! {
                div { class: "kuva-tooltip", style, "{tip.text}" }
            }
        }
        None => rsx! {},
    }
}

/// The consumer-owned interaction signals threaded through the node tree on the
/// interactive render paths. `Copy` (Dioxus signals are `Copy`) and `Default` (all
/// `None`), so it costs nothing to pass down the recursion and the static path just
/// uses `RenderCtx::default()`.
///
/// Both fields are consumer-owned `Signal`s (created with `use_signal`) and passed
/// *in*, never created here — that is what keeps this backend free of the dioxus
/// `hooks` feature (it only *reads/writes* signals and attaches handlers, both in
/// the already-enabled `signals`/`html` features). Each new interaction adds a
/// field here rather than another threaded parameter.
#[derive(Clone, Copy, Default)]
struct RenderCtx {
    /// Hover-tooltip state. `Some` -> titled groups get hover handlers
    /// that write it (and drop the static `<title>`); `None` -> static `<title>`s.
    tooltip: Option<Signal<Option<Tooltip>>>,
    /// The set of *hidden* series labels for the legend toggle. `Some`
    /// -> a data series whose `data-group` id is in the set is dropped from the
    /// render, and a legend-entry group toggles its id in the set on click; `None`
    /// -> every series renders. See [`series_is_hidden`] / [`is_legend_entry`].
    hidden_series: Option<Signal<HashSet<String>>>,
    /// Live coordinate-readout state. `Some` (and the `Scene` carries
    /// `axis_meta`) -> the root `<svg>` gets `onmousemove`/`onmouseleave` handlers
    /// that convert the cursor to data coordinates ([`readout_text`]) and write this
    /// signal; `None` -> no readout (the static render path). The displayed value
    /// rides a `Tooltip` (text + cursor position) shown by a [`KuvaTooltip`] overlay.
    readout: Option<Signal<Option<Tooltip>>>,
}

/// `Scene` -> `Element` serializer. Mirrors `SvgBackend`'s shape (`new()` +
/// `Default`) so the two are drop-in interchangeable at the pipeline seam:
/// `render_multiple(plots, layout)` -> `KuvaDioxus::new().render_scene(&scene)`.
///
/// The optional `class` is applied to the root `<svg>` so a consumer's stylesheet
/// can size the responsive `viewBox`-only chart (e.g. `width:100%;height:auto`).
/// `None` -> no `class` attribute. `background` / `text_color` remap kuva's baked
/// `Scene` colors to host theme tokens or defer them to CSS; both default to
/// `Scene` (parity).
#[derive(Debug, Clone, Default)]
pub struct KuvaDioxus {
    class: Option<String>,
    background: Background,
    text_color: TextColor,
}

impl KuvaDioxus {
    pub const fn new() -> Self {
        Self {
            class: None,
            background: Background::Scene,
            text_color: TextColor::Scene,
        }
    }

    /// Set the CSS class on the root `<svg>`, letting external CSS scale the
    /// `viewBox`-only chart to its container.
    pub fn with_class(mut self, class: impl Into<String>) -> Self {
        self.class = Some(class.into());
        self
    }

    /// Override how the chart background is emitted. Defaults to
    /// [`Background::Scene`] (parity with `SvgBackend`); pass [`Background::None`]
    /// to let the host surface show through, or [`Background::Token`] for a token.
    pub fn with_background(mut self, background: Background) -> Self {
        self.background = background;
        self
    }

    /// Override the root `fill` / text color. Defaults to
    /// [`TextColor::Scene`] (parity with `SvgBackend`); pass [`TextColor::Inherit`]
    /// to let CSS drive it, or [`TextColor::Token`] for a token / `currentColor`.
    pub fn with_text_color(mut self, text_color: TextColor) -> Self {
        self.text_color = text_color;
        self
    }

    /// Resolve the background-rect fill for this `Scene` under the theming config:
    /// the kuva-baked `Scene.background_color` by default, a token, or `None`
    /// (no rect). Pure so the theming logic is unit-tested without a render harness.
    fn background_fill(&self, scene: &Scene) -> Option<String> {
        match &self.background {
            Background::Scene => scene.background_color.clone(),
            Background::None => None,
            Background::Token(color) => Some(color.clone()),
        }
    }

    /// Resolve the root `<svg>` `fill` (text color) for this `Scene` under the
    /// theming config: the kuva-baked `Scene.text_color` by default, a token, or
    /// `None` (no `fill`, inherit from CSS). Pure for the same reason as
    /// [`Self::background_fill`].
    fn text_fill(&self, scene: &Scene) -> Option<String> {
        match &self.text_color {
            TextColor::Scene => scene.text_color.clone(),
            TextColor::Inherit => None,
            TextColor::Token(color) => Some(color.clone()),
        }
    }

    /// Build the root `<svg>` node and map every `Scene` primitive under it.
    ///
    /// The flat `GroupStart/End` + `ClipStart/End` markers in `scene.elements`
    /// are first folded into a nested [`Node`] tree (see [`fold_scene`]); each
    /// node is then emitted as RSX. `Scene.defs` (gradients/markers and the
    /// `<clipPath>` definitions the fold's clip wrappers reference) are raw SVG
    /// strings with no typed-node equivalent, so they are emitted as a single
    /// `<defs>` `dangerous_inner_html` island — the *only* such island in this
    /// backend (`Scene.scripts` is dropped entirely). The root carries a `viewBox`
    /// (from `Scene.width/height`) and `self.class` rather than fixed
    /// `width`/`height`, so it scales to its container; the background rect and
    /// text `fill` resolve via the configured `Background` / `TextColor`.
    pub fn render_scene(&self, scene: &Scene) -> Element {
        self.render_scene_impl(scene, RenderCtx::default())
    }

    /// Like [`Self::render_scene`], but wires native hover tooltips:
    /// each titled `<g>` gets `onmouseenter`/`onmousemove`/`onmouseleave`
    /// handlers that write `tooltip` (and **drops** the static `<title>` child),
    /// replacing the SVG backend's `<title>` + injected hover JS. The `tooltip`
    /// signal is owned by the consumer (`use_signal`) and shared with a
    /// [`KuvaTooltip`] overlay that displays it — so this backend stays
    /// hook-free. Usually reached via [`KuvaPlot`]'s `tooltip` prop rather than
    /// called directly.
    pub fn render_scene_with_tooltip(
        &self,
        scene: &Scene,
        tooltip: Signal<Option<Tooltip>>,
    ) -> Element {
        self.render_scene_impl(
            scene,
            RenderCtx {
                tooltip: Some(tooltip),
                ..RenderCtx::default()
            },
        )
    }

    /// Like [`Self::render_scene`], but wires the native coordinate readout:
    /// when the `Scene` carries `axis_meta`, the root `<svg>` gets
    /// `onmousemove`/`onmouseleave` handlers that convert the cursor position to
    /// data coordinates ([`readout_text`]) and write `readout`, replacing the SVG
    /// backend's injected JS `px2data` readout. The `readout` signal is owned by
    /// the consumer (`use_signal`) and shared with a [`KuvaTooltip`] overlay that
    /// displays it — so this backend stays hook-free. Usually reached via
    /// [`KuvaPlot`]'s `readout` prop rather than called directly.
    pub fn render_scene_with_readout(
        &self,
        scene: &Scene,
        readout: Signal<Option<Tooltip>>,
    ) -> Element {
        self.render_scene_impl(
            scene,
            RenderCtx {
                readout: Some(readout),
                ..RenderCtx::default()
            },
        )
    }

    fn render_scene_impl(&self, scene: &Scene, ctx: RenderCtx) -> Element {
        let tree = fold_scene(&scene.elements);
        let defs_html = defs_inner_html(scene);
        let background = self.background_fill(scene);
        let text_fill = self.text_fill(scene);
        let view_box = view_box(scene);
        let class = self.class.clone();
        let font_family = scene.font_family.clone();
        // Axis calibration for the coordinate readout: present only on
        // interactive axis plots. Snapshotted to a `Copy` `Axes` so the handlers can
        // capture it by value into `'static` closures.
        let axes = scene.axis_meta.as_ref().map(Axes::from_meta);

        // Build the `<svg>` body once and embed it under whichever root variant we
        // pick below — the readout-handler variant or the plain one — so the shared
        // children aren't duplicated.
        let body = rsx! {
            // Full-bleed background rect FIRST so it paints behind everything,
            // mirroring `svg.rs`'s ordering. Resolved via the theming config:
            // Scene.background_color by default, a token, or omitted entirely.
            if let Some(bg) = background {
                rect { width: "100%", height: "100%", fill: bg }
            }
            // `<defs>` next (mirrors `svg.rs`), as an inner-HTML island: the
            // gradient/marker/`clipPath` strings are raw SVG that has no typed
            // RSX form. Emitted only when present.
            if let Some(defs) = defs_html {
                defs { dangerous_inner_html: defs }
            }
            for node in tree.iter() {
                {render_node(node, ctx)}
            }
            // `Scene.scripts` (kuva's baked interactivity JS) is deliberately NOT
            // emitted: inner-HTML `<script>` never auto-executes in the
            // VDOM, native signal-driven interactivity replaces it, and only
            // `CalendarPlot` produces any — see the module docs.
        };

        // viewBox + class, no fixed width/height: the chart scales to its container,
        // sized by the consumer's CSS. Root `fill` (text color) resolved via the
        // theming config.
        match (ctx.readout, axes) {
            // Coordinate readout wired AND the Scene has axis calibration: attach
            // the mousemove/leave handlers to the root `<svg>`. `element_coordinates`
            // is the cursor offset within the `<svg>` in CSS px; that equals the
            // Scene/viewBox space the `axis_meta` is in when the chart renders at its
            // intrinsic size. (A responsively-scaled chart would need the rendered
            // size to correct the scale — a DOM read that requires a `mounted`
            // node-ref and thus the dioxus `hooks` feature, which this backend
            // deliberately avoids; deferred to keep the dep surface at
            // `macro`/`html`/`signals`.)
            (Some(sig), Some(axes)) => rsx! {
                svg {
                    view_box,
                    class,
                    font_family,
                    fill: text_fill,
                    onmousemove: move |evt: MouseEvent| {
                        let p = evt.element_coordinates();
                        let c = evt.client_coordinates();
                        let mut s = sig;
                        s.set(readout_text(axes, p.x, p.y).map(|text| Tooltip {
                            text,
                            x: c.x,
                            y: c.y,
                            // The coordinate readout is never pinned — it tracks the
                            // cursor live (click-to-pin applies only to group titles).
                            pinned: false,
                        }));
                    },
                    onmouseleave: move |_| {
                        let mut s = sig;
                        s.set(None);
                    },
                    {body}
                }
            },
            _ => rsx! {
                svg {
                    view_box,
                    class,
                    font_family,
                    fill: text_fill,
                    {body}
                }
            },
        }
    }
}

/// The `viewBox` value for the root `<svg>`: `"0 0 <width> <height>"` with the
/// `Scene` dimensions via [`fmt_float`]. Emitting this (and dropping the fixed
/// `width`/`height` the SVG backend bakes in) lets the chart scale to its
/// container under the consumer's CSS — the common responsive-SVG convention
/// (`view_box: "0 0 W H"`).
fn view_box(scene: &Scene) -> String {
    format!("0 0 {} {}", fmt_float(scene.width), fmt_float(scene.height))
}

/// Run kuva's public `plot -> Scene` pipeline: lay the plots out (with the given
/// `Layout`, or an auto layout derived from the plots when `None`) and rasterise
/// them into a [`Scene`]. This is the *pure*, harness-free half of [`KuvaPlot`] —
/// it taps the exact seam `SvgBackend` consumers tap, just stopping at the
/// `Scene` so the unit tests can assert on it directly (an `Element` can't be).
///
/// Sizing flows through the `Layout` (`Layout::with_width`/`with_height`, else
/// kuva's defaults) and is baked into `Scene.width/height`; the responsive
/// `view_box` that lets a chart scale to its container is applied at render time
/// by [`KuvaDioxus`].
pub fn scene_from_plots(plots: Vec<Plot>, layout: Option<Layout>) -> Scene {
    let layout = layout.unwrap_or_else(|| Layout::auto_from_plots(&plots));
    render_multiple(plots, layout)
}

/// A thin Dioxus component that draws a kuva plot through the native RSX backend,
/// regenerating the [`Scene`] reactively — only when its `data` changes.
///
/// `data` is the reactive *input* the plot is a function of (data points, a scale,
/// a filter set, …); `builder` is a **pure** `data -> Scene` mapping, typically
/// `|d| scene_from_plots(plots_from(d), layout)`. On each change of `data` the
/// component reruns `builder` and renders the result via [`KuvaDioxus`].
///
/// ## Why this shape gives fine-grained updates
///
/// `Scene` is neither `Clone` nor `PartialEq` (and this backend does not edit it),
/// so a `Scene` can't live in a `use_memo`/`Memo<Scene>` *nor* sit on a
/// `#[component]` prop. Instead the keying rides on `data`:
///
/// - `data: ReadSignal<D>` is recognised by the props derive as a **signal prop**,
///   which memoizes by the *inner value* (`D: PartialEq`). So `KuvaPlot`
///   re-renders — and thus rebuilds the `Scene` — **only when `data` actually
///   changes**, not on unrelated parent re-renders. The component is itself the
///   memo boundary, keyed on the data signal; that is the realisation of "regen
///   the `Scene` in a `use_memo` keyed on data signals" without a literal
///   `Memo<Scene>` (impossible here) and without pulling dioxus's `hooks` feature
///   (keeping the wasm dep surface to `macro`/`html`/`signals`).
/// - `builder: Callback<D, Scene>` is a **callback prop**: the derive points it to
///   the latest closure in place and never treats it as "changed", so handing
///   `KuvaPlot` a freshly-created closure each parent render does *not* force a
///   rebuild. (`Plot`/`Scene` aren't `Clone`/`PartialEq`, so they can't ride on a
///   prop directly — the caller owns them inside the closure.)
///
/// `data` accepts a `Signal`, a `Memo`, or a plain value (`ReadSignal`
/// auto-converts), so a static chart just passes its data by value. Typical use:
///
/// ```rust,ignore
/// let points = use_signal(|| vec![(1.0, 2.0), (2.0, 3.0)]);
/// rsx! {
///     KuvaPlot {
///         data: points,
///         builder: move |pts: Vec<(f64, f64)>| scene_from_plots(
///             vec![Plot::Scatter(ScatterPlot::new().with_data(pts))],
///             Some(Layout::auto_from_plots(&[]).with_width(640.0).with_height(400.0)),
///         ),
///     }
/// }
/// ```
///
/// (The prop is named `builder`, not `build`, because the `#[component]` props
/// derive reserves a `build()` method for its own builder.)
///
/// The optional `class` is applied to the root `<svg>` so external CSS can size
/// the responsive (`viewBox`-only) chart to its container. Omit it for an
/// unstyled, container-filling chart.
///
/// `background` / `text_color` remap kuva's baked `Scene` colors to host theme
/// tokens or defer them to CSS; both default to `Scene` (parity with
/// `SvgBackend`). A host surface that owns its own background typically pairs
/// `background: Background::None` with `text_color: TextColor::Inherit` + a `class`.
/// `tooltip` opts the plot into native hover tooltips: pass a
/// consumer-owned `use_signal(|| None::<Tooltip>)` and the titled groups get
/// `onmouseenter`/`onmousemove`/`onmouseleave` handlers (replacing the static
/// `<title>`), with a [`KuvaTooltip`] overlay rendered alongside the `<svg>` to
/// display it. Omit it for the static render (browser-native `<title>` tooltips).
/// The consumer owns the signal so this component — and the whole backend — stays
/// free of the dioxus `hooks` feature. The same titled groups also support
/// **click-to-pin**: clicking a group pins its tooltip (frozen until
/// clicked again), so it stays put while you read or compare it — no extra prop,
/// it rides the same `tooltip` signal (`Tooltip::pinned`).
///
/// `hidden_series` opts the plot into a native legend toggle: pass a
/// consumer-owned `use_signal(HashSet::<String>::new)` of *hidden* series labels.
/// A plotted series whose label is in the set is dropped from the render, and each
/// legend swatch becomes a click target that toggles its series in the set (and
/// dims while hidden). The series/legend grouping it relies on exists only when the
/// `Scene` is built with `Layout::with_interactive()`; omit the prop (or build a
/// non-interactive layout) for a plain chart. The consumer owns the signal, so —
/// like `tooltip` — this keeps the backend free of the dioxus `hooks` feature.
///
/// `readout` opts the plot into a live coordinate readout: pass a
/// consumer-owned `use_signal(|| None::<Tooltip>)`. When the `Scene` carries
/// `axis_meta` (interactive axis plots), the root `<svg>` gets an `onmousemove`
/// handler that converts the cursor to data coordinates and writes the signal,
/// shown by a [`KuvaTooltip`] overlay — the native replacement for kuva's injected
/// JS `px2data` readout. The conversion is exact when the chart renders at its
/// intrinsic (`viewBox`) size; a responsively-scaled chart would need a DOM read to
/// correct the scale, which is deferred to keep the backend `hooks`-free. Like the
/// other interaction signals, the consumer owns it.
#[component]
pub fn KuvaPlot<D: Clone + PartialEq + 'static>(
    data: ReadSignal<D>,
    builder: Callback<D, Scene>,
    class: Option<String>,
    #[props(default)] background: Background,
    #[props(default)] text_color: TextColor,
    tooltip: Option<Signal<Option<Tooltip>>>,
    hidden_series: Option<Signal<HashSet<String>>>,
    readout: Option<Signal<Option<Tooltip>>>,
) -> Element {
    // Reading `data()` subscribes this component to the signal; combined with the
    // signal-prop value-memoization above, the `Scene` is rebuilt only when the
    // data changes. The rebuild is correct precisely because `builder`/`Scene` is
    // a pure function of `data` (pinned by `scene_from_plots_is_a_pure_function_*`).
    let scene = builder.call(data());
    let mut backend = KuvaDioxus::new()
        .with_background(background)
        .with_text_color(text_color);
    if let Some(class) = class {
        backend = backend.with_class(class);
    }
    // One render path for every interaction combination: the ctx carries whichever
    // signals the consumer wired (all default to `None` -> a static render).
    let svg = backend.render_scene_impl(
        &scene,
        RenderCtx {
            tooltip,
            hidden_series,
            readout,
        },
    );
    // Emit the svg plus one overlay per wired display signal, as a fragment (no
    // wrapper element, so the consumer's sizing/layout around the chart is
    // undisturbed). Both overlays reuse `KuvaTooltip`: the readout's value is just a
    // coordinate string riding the same `Tooltip` (text + cursor position) shape.
    rsx! {
        {svg}
        if let Some(sig) = tooltip {
            KuvaTooltip { tooltip: sig }
        }
        if let Some(sig) = readout {
            KuvaTooltip { tooltip: sig }
        }
    }
}

/// The concatenated inner HTML for the `<defs>` island, or `None` when there is
/// nothing to emit. Mirrors `svg.rs`, which writes each `Scene.defs` entry
/// verbatim with no separator. The interactive `<style>` block `svg.rs` also
/// places in `<defs>` (gated on `interactive`/`has_tooltips`) is **not** emitted
/// here — this backend drives interactivity natively (signals + handlers), so the
/// `<defs>` island carries only the raw gradient/marker/`clipPath` definitions.
fn defs_inner_html(scene: &Scene) -> Option<String> {
    if scene.defs.is_empty() {
        None
    } else {
        Some(scene.defs.concat())
    }
}

/// Nesting tree built by [`fold_scene`] from the flat primitive stream, before
/// RSX emission. Holding `&Primitive`/`&str` borrows (no clones) keeps the fold
/// cheap; `render_scene` builds it as a local that outlives the `rsx!` body.
enum Node<'a> {
    /// Any non-marker primitive — emitted by [`render_primitive`].
    Leaf(&'a Primitive),
    /// A `GroupStart`..`GroupEnd` span -> `<g>` wrapper.
    Group {
        transform: &'a Option<String>,
        title: &'a Option<String>,
        extra_attrs: &'a Option<String>,
        children: Vec<Node<'a>>,
    },
    /// A `ClipStart`..`ClipEnd` span -> `<g clip-path="url(#id)">` wrapper. The
    /// referenced `<clipPath>` *definition* lives in `Scene.defs` (emitted by the
    /// defs island), not here — same split as `svg.rs`.
    Clip {
        id: &'a str,
        children: Vec<Node<'a>>,
    },
}

/// Fold the flat `GroupStart/End` + `ClipStart/End` marker stream into a nested
/// [`Node`] tree. Pure and introspectable so the degenerate cases are unit-tested
/// directly (an `Element` can't be).
fn fold_scene(elems: &[Primitive]) -> Vec<Node<'_>> {
    let mut idx = 0;
    fold_level(elems, &mut idx, false)
}

/// Recursive-descent worker for [`fold_scene`]. Consumes from `*idx`, building
/// one nesting level. `nested` is `true` inside a group/clip span: there a close
/// marker (`GroupEnd`/`ClipEnd`, treated interchangeably as `svg.rs` does) ends
/// the level and is consumed; at top level (`nested == false`) a stray close is
/// skipped defensively. A start with no matching close folds to end-of-input
/// (its children are whatever follows) — a malformed `Scene` renders, never
/// panics.
fn fold_level<'a>(elems: &'a [Primitive], idx: &mut usize, nested: bool) -> Vec<Node<'a>> {
    let mut out = Vec::new();
    while *idx < elems.len() {
        match &elems[*idx] {
            Primitive::GroupStart {
                transform,
                title,
                extra_attrs,
            } => {
                *idx += 1; // consume the GroupStart
                let children = fold_level(elems, idx, true);
                out.push(Node::Group {
                    transform,
                    title,
                    extra_attrs,
                    children,
                });
            }
            Primitive::ClipStart { id, .. } => {
                *idx += 1; // consume the ClipStart
                let children = fold_level(elems, idx, true);
                out.push(Node::Clip { id, children });
            }
            Primitive::GroupEnd | Primitive::ClipEnd => {
                *idx += 1; // consume the close marker either way
                if nested {
                    return out; // ...and return this level to the parent
                }
                // Stray close at top level: skip it, keep going.
            }
            other => {
                out.push(Node::Leaf(other));
                *idx += 1;
            }
        }
    }
    out
}

/// The `class` attribute value for a group `<g>`, matching `svg.rs`: a
/// `class="..."` inside `extra_attrs` wins; otherwise a `Some` `title` yields
/// `class="tt"`; otherwise no class.
///
/// `extra_attrs`' non-`class` pairs (`data-*`) are **not** emitted as DOM
/// attributes here. They are produced only in interactive mode (gated on
/// `computed.interactive`) and read only by injected JS, while Dioxus
/// `Attribute::new` requires a `&'static str` name — a verbatim runtime attr
/// blob can't be spread without leaking. The native interactivity here reads the
/// structured `Scene` metadata (`axis_meta`, group titles) directly instead of
/// round-tripping `data-*`, so dropping them from the static render is correct,
/// not a shortcut.
fn group_class(title: &Option<String>, extra_attrs: &Option<String>) -> Option<String> {
    if let Some(attrs) = extra_attrs {
        if let Some(class) = extract_attr_value(attrs, "class") {
            return Some(class);
        }
    }
    title.as_ref().map(|_| "tt".to_string())
}

/// Pull the value out of a `name="value"` pair in a verbatim SVG attribute blob
/// (e.g. `class="tt" data-group="3"`). Matches the exact `name="` token kuva's
/// producers emit; returns `None` if absent or if the closing quote is missing.
fn extract_attr_value(blob: &str, name: &str) -> Option<String> {
    let key = format!("{name}=\"");
    let start = blob.find(&key)? + key.len();
    let end = blob[start..].find('"')? + start;
    Some(blob[start..end].to_string())
}

/// Emit one [`Node`] as RSX. Groups/clips recurse into their children; leaves
/// defer to [`render_primitive`]. `ctx` carries the consumer-owned interaction
/// signals: a titled group gets hover handlers writing `ctx.tooltip`
/// instead of a static `<title>`; and when `ctx.hidden_series` is
/// wired a hidden data series is dropped and a legend entry toggles
/// its series on click. Static (`RenderCtx::default()`) renders are unchanged.
fn render_node(node: &Node, ctx: RenderCtx) -> Element {
    match node {
        Node::Leaf(prim) => render_primitive(prim),
        Node::Group {
            transform,
            title,
            extra_attrs,
            children,
        } => {
            let class = group_class(title, extra_attrs);
            let transform = (*transform).clone();
            // Legend toggle: only when a hidden-series signal is wired.
            // A *data* series whose `data-group` id is currently hidden is dropped
            // entirely; a *legend-entry* group instead becomes a click target that
            // toggles its series in the set (and dims while hidden). Both group
            // kinds carry `data-group` (added by kuva only in interactive mode);
            // the legend entry is distinguished by `class="legend-entry"`.
            if let Some(hidden_sig) = ctx.hidden_series {
                if series_is_hidden(extra_attrs, &hidden_sig.read()) {
                    return rsx! {}; // filtered out — neither the series nor its <title>
                }
                if is_legend_entry(extra_attrs) {
                    if let Some(id) = group_data_group(extra_attrs) {
                        let is_hidden = hidden_sig.read().contains(&id);
                        return rsx! {
                            g {
                                class,
                                transform,
                                style: "cursor: pointer;",
                                // Dim the swatch while its series is toggled off, so
                                // the toggle state is visible without re-rendering JS.
                                opacity: if is_hidden { Some("0.4") } else { None },
                                onclick: move |_| {
                                    let mut sig = hidden_sig;
                                    let mut set = sig.write();
                                    // Toggle: remove if present (show), else insert (hide).
                                    if !set.remove(&id) {
                                        set.insert(id.clone());
                                    }
                                },
                                for child in children.iter() {
                                    {render_node(child, ctx)}
                                }
                            }
                        };
                    }
                }
            }
            // Interactive tooltips only when enabled AND the group
            // carries a title (the tooltip text) — matching svg.rs's `has_tooltips`
            // gate. Otherwise fall through to the static `<title>` form.
            match (
                ctx.tooltip,
                group_tooltip_text(title, ctx.tooltip.is_some()),
            ) {
                (Some(sig), Some(text)) => {
                    let text = text.to_string();
                    rsx! {
                        g {
                            class,
                            transform,
                            // The group is a click target (click-to-pin).
                            style: "cursor: pointer;",
                            // enter/move set text + cursor (client) position; leave
                            // clears. Replaces the static `<title>`. All three are
                            // gated on `hover_locked`: a pinned tooltip is frozen.
                            onmouseenter: {
                                let text = text.clone();
                                move |evt: MouseEvent| {
                                    let mut s = sig;
                                    if hover_locked(&s.read()) {
                                        return;
                                    }
                                    let c = evt.client_coordinates();
                                    s.set(Some(Tooltip { text: text.clone(), x: c.x, y: c.y, pinned: false }));
                                }
                            },
                            onmousemove: {
                                let text = text.clone();
                                move |evt: MouseEvent| {
                                    let mut s = sig;
                                    if hover_locked(&s.read()) {
                                        return;
                                    }
                                    let c = evt.client_coordinates();
                                    s.set(Some(Tooltip { text: text.clone(), x: c.x, y: c.y, pinned: false }));
                                }
                            },
                            onmouseleave: move |_| {
                                let mut s = sig;
                                if hover_locked(&s.read()) {
                                    return;
                                }
                                s.set(None);
                            },
                            // Click-to-pin: toggle this group's tooltip between
                            // pinned (frozen at the click position) and cleared.
                            onclick: {
                                let text = text.clone();
                                move |evt: MouseEvent| {
                                    let c = evt.client_coordinates();
                                    let mut s = sig;
                                    let next = {
                                        let cur = s.read();
                                        pin_on_click(&cur, &text, c.x, c.y)
                                    };
                                    s.set(next);
                                }
                            },
                            for child in children.iter() {
                                {render_node(child, ctx)}
                            }
                        }
                    }
                }
                _ => rsx! {
                    g {
                        class,
                        transform,
                        // `<title>` is the group's first child, as in `svg.rs`
                        // (auto-escaped, no math lowering — matching `write_escaped`).
                        if let Some(tip) = title {
                            title { "{tip}" }
                        }
                        for child in children.iter() {
                            {render_node(child, ctx)}
                        }
                    }
                },
            }
        }
        Node::Clip { id, children } => rsx! {
            g {
                clip_path: format!("url(#{id})"),
                for child in children.iter() {
                    {render_node(child, ctx)}
                }
            }
        },
    }
}

/// The `data-group` value (the series label) a group carries, if any. Both data
/// series groups (`class="tt" data-group="<label>"`, [`render.rs` `add_line`]) and
/// legend-entry groups (`class="legend-entry" data-group="<label>"`) carry it with
/// the *same* label, which is how a legend entry and its series correspond. Present
/// only when the `Scene` was built in interactive mode (`Layout::with_interactive`).
fn group_data_group(extra_attrs: &Option<String>) -> Option<String> {
    extra_attrs
        .as_ref()
        .and_then(|a| extract_attr_value(a, "data-group"))
}

/// Whether a group is a legend swatch entry (`class="legend-entry"`) rather than a
/// plotted data series. Legend entries are the click targets that toggle a series;
/// they are never filtered out (so a hidden series can be toggled back on).
fn is_legend_entry(extra_attrs: &Option<String>) -> bool {
    extra_attrs
        .as_ref()
        .and_then(|a| extract_attr_value(a, "class"))
        .as_deref()
        == Some("legend-entry")
}

/// Whether a data-series group should be dropped from the render: it carries a
/// `data-group` id, is **not** a legend entry, and that id is in the `hidden` set.
/// Pure, so the legend filter decision is unit-tested without a render harness.
fn series_is_hidden(extra_attrs: &Option<String>, hidden: &HashSet<String>) -> bool {
    if is_legend_entry(extra_attrs) {
        return false; // keep the legend swatch so the series can be toggled back on
    }
    group_data_group(extra_attrs).is_some_and(|id| hidden.contains(&id))
}

/// The hover-tooltip text for a group: its `GroupStart.title`, but only when
/// interactive tooltips are `enabled`. Returns `None` when disabled (the static
/// path then emits a `<title>` child instead) or when the group has no title.
/// Pure, so the interactive-vs-static decision is unit-tested without a render
/// harness (the rendered `Element` is opaque).
fn group_tooltip_text(title: &Option<String>, enabled: bool) -> Option<&str> {
    if enabled {
        title.as_deref()
    } else {
        None
    }
}

/// The tooltip state after a titled group is **clicked** (click-to-pin):
///
/// - clicking the group that is *already pinned* (same `text`) unpins it,
///   clearing the overlay (`None`);
/// - any other click — first click, a click while unpinned, or a click on a
///   *different* group while another is pinned — pins this group's tooltip at the
///   click position (`Some(Tooltip { pinned: true, .. })`).
///
/// Pure, so the toggle is unit-tested without a render harness (the rendered `<g>`
/// + its `onclick` are opaque).
fn pin_on_click(current: &Option<Tooltip>, text: &str, x: f64, y: f64) -> Option<Tooltip> {
    match current {
        Some(t) if t.pinned && t.text == text => None,
        _ => Some(Tooltip {
            text: text.to_string(),
            x,
            y,
            pinned: true,
        }),
    }
}

/// Whether the hover handlers (`onmouseenter`/`onmousemove`/`onmouseleave`) must
/// leave the tooltip untouched: a *pinned* tooltip is frozen until clicked again,
/// so hover neither updates its position/text nor clears it. Pure, mirrors the
/// gate the handlers apply before writing the signal.
fn hover_locked(current: &Option<Tooltip>) -> bool {
    matches!(current, Some(t) if t.pinned)
}

/// Map one *leaf* `Primitive` to a typed RSX node. The `GroupStart/End` +
/// `ClipStart/End` markers never reach here — [`fold_scene`] consumes them into
/// [`Node::Group`]/[`Node::Clip`] wrappers, so the catch-all below is dead for
/// those variants but keeps the match total (and renders a partial `Scene`
/// rather than panicking if a new variant is ever added upstream).
fn render_primitive(elem: &Primitive) -> Element {
    match elem {
        Primitive::Circle {
            cx,
            cy,
            r,
            fill,
            fill_opacity,
            stroke,
            stroke_width,
        } => rsx! {
            circle {
                cx: fmt_float(*cx),
                cy: fmt_float(*cy),
                r: fmt_float(*r),
                fill: fill.to_svg_string(),
                fill_opacity: (*fill_opacity).map(fmt_float),
                stroke: stroke.as_ref().map(Color::to_svg_string),
                stroke_width: (*stroke_width).map(fmt_float),
            }
        },
        Primitive::Rect {
            x,
            y,
            width,
            height,
            fill,
            stroke,
            stroke_width,
            opacity,
        } => rsx! {
            rect {
                x: fmt_float(*x),
                y: fmt_float(*y),
                width: fmt_float(*width),
                height: fmt_float(*height),
                fill: fill.to_svg_string(),
                stroke: stroke.as_ref().map(Color::to_svg_string),
                stroke_width: (*stroke_width).map(fmt_float),
                fill_opacity: (*opacity).map(fmt_float),
            }
        },
        Primitive::Text {
            x,
            y,
            content,
            size,
            anchor,
            rotate,
            bold,
            color,
        } => rsx! {
            text {
                x: fmt_float(*x),
                y: fmt_float(*y),
                font_size: size.to_string(),
                text_anchor: anchor_str(anchor),
                font_weight: if *bold { Some("bold") } else { None },
                fill: color.as_ref().map(Color::to_svg_string),
                transform: (*rotate).map(|a| rotate_transform(a, *x, *y)),
                {text_content(content)}
            }
        },
        Primitive::Line {
            x1,
            y1,
            x2,
            y2,
            stroke,
            stroke_width,
            stroke_dasharray,
        } => rsx! {
            line {
                x1: fmt_float(*x1),
                y1: fmt_float(*y1),
                x2: fmt_float(*x2),
                y2: fmt_float(*y2),
                stroke: stroke.to_svg_string(),
                stroke_width: fmt_float(*stroke_width),
                stroke_dasharray: stroke_dasharray.clone(),
            }
        },
        // SVG backend emits a `<path d="M x y L x y …">` with `fill="none"`,
        // *not* a `<polyline>` — match that so the golden-diff stays clean.
        Primitive::PolyLine {
            points,
            stroke,
            stroke_width,
            stroke_dasharray,
        } => rsx! {
            path {
                d: polyline_path_d(points),
                stroke: stroke.to_svg_string(),
                stroke_width: fmt_float(*stroke_width),
                fill: "none",
                stroke_dasharray: stroke_dasharray.clone(),
            }
        },
        Primitive::Path(pd) => rsx! {
            path {
                d: pd.d.clone(),
                stroke: pd.stroke.to_svg_string(),
                stroke_width: fmt_float(pd.stroke_width),
                // `fill="none"` when no fill is set, mirroring `svg.rs`.
                fill: pd.fill.as_ref().map_or_else(|| "none".to_string(), Color::to_svg_string),
                fill_opacity: pd.opacity.map(fmt_float),
                stroke_dasharray: pd.stroke_dasharray.clone(),
            }
        },
        Primitive::RichText {
            x,
            y,
            spans,
            size,
            anchor,
            color,
        } => rsx! {
            text {
                x: fmt_float(*x),
                y: fmt_float(*y),
                font_size: size.to_string(),
                text_anchor: anchor_str(anchor),
                fill: color.as_ref().map(Color::to_svg_string),
                for span in spans.iter() {
                    {render_span(span)}
                }
            }
        },
        // Struct-of-arrays batch: one `<circle>` per index, sharing `r`/`fill`/
        // `fill_opacity`/`stroke`/`stroke_width`. (An inner-HTML blob form for
        // very large batches — >>1k points — could be a future opt-in path.)
        Primitive::CircleBatch {
            cx,
            cy,
            r,
            fill,
            fill_opacity,
            stroke,
            stroke_width,
        } => {
            let fill = fill.to_svg_string();
            let stroke = stroke.as_ref().map(Color::to_svg_string);
            rsx! {
                for i in 0..cx.len() {
                    circle {
                        key: "{i}",
                        cx: fmt_float(cx[i]),
                        cy: fmt_float(cy[i]),
                        r: fmt_float(*r),
                        fill: fill.clone(),
                        fill_opacity: (*fill_opacity).map(fmt_float),
                        stroke: stroke.clone(),
                        stroke_width: (*stroke_width).map(fmt_float),
                    }
                }
            }
        }
        // Struct-of-arrays batch: one `<rect>` per index, each with its own fill.
        Primitive::RectBatch { x, y, w, h, fills } => rsx! {
            for i in 0..x.len() {
                rect {
                    key: "{i}",
                    x: fmt_float(x[i]),
                    y: fmt_float(y[i]),
                    width: fmt_float(w[i]),
                    height: fmt_float(h[i]),
                    fill: fills[i].to_svg_string(),
                }
            }
        },
        // GroupStart/End + ClipStart/End are folded out by `fold_scene` and
        // never reach here; the arm only keeps the match total.
        _ => rsx! {},
    }
}

#[cfg(test)]
mod tests {
    //! Parity tests for the leaf-primitive mapping.
    //!
    //! Following the consumer's `chart.rs` convention, we assert on computed
    //! values, not on a rendered VDOM (no render harness). The strongest guard we
    //! can write without one is: the attribute fragments these helpers produce
    //! must appear verbatim in the *live* `SvgBackend` output for the same
    //! `Primitive`. That ties `fmt_float`/`Color`/`text_content` to the real
    //! backend, so the two serializers cannot silently diverge on formatting.

    use super::*;
    use crate::backend::svg::SvgBackend;
    use crate::render::render::Scene;

    /// Render a one-primitive `Scene` through the SVG backend.
    fn svg_of(prim: Primitive) -> String {
        let mut scene = Scene::new(100.0, 100.0);
        scene.elements.push(prim);
        SvgBackend::new().render_scene(&scene)
    }

    // ── fmt_float: the write_float contract (round 2dp, strip whole .0) ──────

    #[test]
    fn fmt_float_strips_whole_numbers_and_rounds_to_two_dp() {
        assert_eq!(fmt_float(3.0), "3");
        assert_eq!(fmt_float(-0.0), "0");
        assert_eq!(fmt_float(3.14159), "3.14"); // rounded to 2dp
        assert_eq!(fmt_float(1.5), "1.5");
        assert_eq!(fmt_float(10.0), "10");
    }

    // ── Color -> CSS string (reused Color::to_svg_string) ────────────────────

    #[test]
    fn color_css_matches_svg_serialization() {
        assert_eq!(Color::Rgb(70, 130, 180).to_svg_string(), "#4682b4");
        assert_eq!(Color::None.to_svg_string(), "none");
        assert_eq!(
            Color::Css("currentColor".into()).to_svg_string(),
            "currentColor"
        );
    }

    #[test]
    fn anchor_keywords_match() {
        assert_eq!(anchor_str(&TextAnchor::Start), "start");
        assert_eq!(anchor_str(&TextAnchor::Middle), "middle");
        assert_eq!(anchor_str(&TextAnchor::End), "end");
    }

    // ── Responsive viewBox ───────────────────────────────────────────────────

    #[test]
    fn view_box_is_zero_origin_scene_dimensions() {
        // The root <svg> carries `viewBox="0 0 W H"` (the responsive-SVG convention),
        // sourced from Scene.width/height — the same values SvgBackend would bake
        // into fixed width/height attrs.
        let scene = Scene::new(640.0, 400.0);
        assert_eq!(view_box(&scene), "0 0 640 400");
    }

    #[test]
    fn view_box_formats_dimensions_via_fmt_float() {
        // Fractional dimensions round to 2dp via fmt_float, matching how the SVG
        // backend would have serialized width/height — so a viewBox-vs-width/height
        // golden-diff stays consistent on the numbers.
        let scene = Scene::new(640.5, 400.125);
        assert_eq!(view_box(&scene), "0 0 640.5 400.13");
    }

    #[test]
    fn view_box_matches_the_svg_backend_dimensions() {
        // The SVG backend bakes the same Scene dimensions into width="" height="";
        // our viewBox must carry the identical numbers (just relocated), so the two
        // backends agree on the intrinsic size even though only ours is responsive.
        let scene = Scene::new(320.0, 240.0);
        let svg = SvgBackend::new().render_scene(&scene);
        assert!(svg.contains(r#"width="320""#), "{svg}");
        assert!(svg.contains(r#"height="240""#), "{svg}");
        assert_eq!(view_box(&scene), "0 0 320 240");
    }

    // ── Theming: background + text-color resolution ──────────────────────────
    //
    // The rendered `<svg>` is opaque, so assert on the pure resolution helpers
    // (`background_fill`/`text_fill`) that decide the root `fill` and whether a
    // full-bleed background rect is emitted — and pin the default to `SvgBackend`
    // parity.

    #[test]
    fn background_defaults_to_scene_color_for_svg_parity() {
        // Default `Background::Scene` resolves to Scene.background_color verbatim —
        // the same full-bleed fill SvgBackend bakes into its <rect width="100%">.
        let scene = Scene::new(10.0, 10.0); // Scene::new defaults to Some("white")
        assert_eq!(
            KuvaDioxus::new().background_fill(&scene).as_deref(),
            Some("white")
        );
        // SvgBackend really emits that rect, so the two agree by default.
        let svg = SvgBackend::new().render_scene(&scene);
        assert!(
            svg.contains(r#"<rect width="100%" height="100%" fill="white""#),
            "{svg}"
        );
    }

    #[test]
    fn background_none_in_scene_emits_no_rect_under_default_mode() {
        // The Scene background is *already* optional (Scene::with_background(None));
        // when it's None, the default mode emits no rect either.
        let scene = Scene::new(10.0, 10.0).with_background(None);
        assert_eq!(KuvaDioxus::new().background_fill(&scene), None);
    }

    #[test]
    fn with_background_none_drops_the_rect_regardless_of_scene() {
        let scene = Scene::new(10.0, 10.0); // Some("white")
        let backend = KuvaDioxus::new().with_background(Background::None);
        assert_eq!(backend.background_fill(&scene), None);
    }

    #[test]
    fn with_background_token_overrides_the_scene_color() {
        let scene = Scene::new(10.0, 10.0); // Some("white")
        let backend = KuvaDioxus::new().with_background(Background::Token("var(--surface)".into()));
        assert_eq!(
            backend.background_fill(&scene).as_deref(),
            Some("var(--surface)")
        );
    }

    #[test]
    fn text_color_defaults_to_scene_color() {
        let mut scene = Scene::new(10.0, 10.0);
        scene.text_color = Some("black".into());
        assert_eq!(
            KuvaDioxus::new().text_fill(&scene).as_deref(),
            Some("black")
        );
        // A Scene with no text_color -> no `fill` (matches Scene::new's None).
        assert_eq!(KuvaDioxus::new().text_fill(&Scene::new(10.0, 10.0)), None);
    }

    #[test]
    fn with_text_color_inherit_drops_the_fill() {
        let mut scene = Scene::new(10.0, 10.0);
        scene.text_color = Some("black".into());
        assert_eq!(
            KuvaDioxus::new()
                .with_text_color(TextColor::Inherit)
                .text_fill(&scene),
            None
        );
    }

    #[test]
    fn with_text_color_token_overrides_the_scene_color() {
        let mut scene = Scene::new(10.0, 10.0);
        scene.text_color = Some("black".into());
        assert_eq!(
            KuvaDioxus::new()
                .with_text_color(TextColor::Token("currentColor".into()))
                .text_fill(&scene)
                .as_deref(),
            Some("currentColor")
        );
    }

    // ── Per-primitive parity against the live SvgBackend output ──────────────

    #[test]
    fn circle_attribute_values_match_svg_backend() {
        let svg = svg_of(Primitive::Circle {
            cx: 12.5,
            cy: 7.0,
            r: 2.25,
            fill: Color::Rgb(255, 0, 0),
            fill_opacity: Some(0.5),
            stroke: Some(Color::Rgb(0, 0, 0)),
            stroke_width: Some(1.0),
        });
        // Each fragment is exactly what render_primitive feeds the circle node.
        assert!(svg.contains(r#"cx="12.5""#), "{svg}");
        assert!(svg.contains(r#"cy="7""#), "{svg}"); // whole .0 stripped
        assert!(svg.contains(r#"r="2.25""#), "{svg}");
        assert!(svg.contains(r##"fill="#ff0000""##), "{svg}");
        assert!(svg.contains(r#"fill-opacity="0.5""#), "{svg}");
        assert!(svg.contains(r##"stroke="#000000""##), "{svg}");
        assert!(svg.contains(r#"stroke-width="1""#), "{svg}");
    }

    #[test]
    fn rect_attribute_values_match_svg_backend() {
        let svg = svg_of(Primitive::Rect {
            x: 1.0,
            y: 2.0,
            width: 30.0,
            height: 40.5,
            fill: Color::Rgb(16, 32, 48),
            stroke: None,
            stroke_width: None,
            opacity: Some(0.25),
        });
        assert!(svg.contains(r#"x="1""#), "{svg}");
        assert!(svg.contains(r#"y="2""#), "{svg}");
        assert!(svg.contains(r#"width="30""#), "{svg}");
        assert!(svg.contains(r#"height="40.5""#), "{svg}");
        assert!(svg.contains(r##"fill="#102030""##), "{svg}");
        assert!(svg.contains(r#"fill-opacity="0.25""#), "{svg}");
        // No stroke supplied -> our Option maps to None -> attr omitted.
        assert!(!svg.contains("stroke="), "{svg}");
    }

    #[test]
    fn text_attribute_values_and_anchor_match_svg_backend() {
        let svg = svg_of(Primitive::Text {
            x: 5.0,
            y: 6.0,
            content: "hello".to_string(),
            size: 14,
            anchor: TextAnchor::Middle,
            rotate: Some(45.0),
            bold: true,
            color: Some(Color::Rgb(0, 0, 0)),
        });
        assert!(svg.contains(r#"x="5""#), "{svg}");
        assert!(svg.contains(r#"y="6""#), "{svg}");
        assert!(svg.contains(r#"font-size="14""#), "{svg}"); // raw u32, not write_float
        assert!(svg.contains(r#"text-anchor="middle""#), "{svg}");
        assert!(svg.contains(r#"font-weight="bold""#), "{svg}");
        assert!(svg.contains(r#"transform="rotate(45,5,6)""#), "{svg}");
        assert_eq!(rotate_transform(45.0, 5.0, 6.0), "rotate(45,5,6)");
        assert!(svg.contains(">hello<"), "{svg}");
    }

    #[test]
    fn line_attribute_values_match_svg_backend() {
        let svg = svg_of(Primitive::Line {
            x1: 1.0,
            y1: 2.5,
            x2: 30.0,
            y2: 4.0,
            stroke: Color::Rgb(0, 128, 255),
            stroke_width: 2.0,
            stroke_dasharray: Some("4 2".to_string()),
        });
        assert!(svg.contains(r#"x1="1""#), "{svg}");
        assert!(svg.contains(r#"y1="2.5""#), "{svg}");
        assert!(svg.contains(r#"x2="30""#), "{svg}");
        assert!(svg.contains(r#"y2="4""#), "{svg}");
        assert!(svg.contains(r##"stroke="#0080ff""##), "{svg}");
        assert!(svg.contains(r#"stroke-width="2""#), "{svg}");
        assert!(svg.contains(r#"stroke-dasharray="4 2""#), "{svg}");
    }

    #[test]
    fn line_without_dasharray_omits_the_attr() {
        let svg = svg_of(Primitive::Line {
            x1: 0.0,
            y1: 0.0,
            x2: 1.0,
            y2: 1.0,
            stroke: Color::Rgb(0, 0, 0),
            stroke_width: 1.0,
            stroke_dasharray: None,
        });
        assert!(!svg.contains("stroke-dasharray="), "{svg}");
    }

    #[test]
    fn polyline_emits_path_d_with_m_l_format() {
        // The d-string our helper builds must appear verbatim in the SVG output —
        // the SVG backend serializes PolyLine as a <path d="M x y L x y …">.
        let points = vec![(1.0, 2.0), (3.5, 4.0), (5.0, 6.0)];
        let d = polyline_path_d(&points);
        assert_eq!(d, "M 1 2 L 3.5 4 L 5 6 "); // trailing space, M then L's

        let svg = svg_of(Primitive::PolyLine {
            points,
            stroke: Color::Rgb(255, 0, 0),
            stroke_width: 1.5,
            stroke_dasharray: None,
        });
        assert!(svg.contains(&format!(r#"d="{d}""#)), "{svg}");
        assert!(svg.contains(r#"fill="none""#), "{svg}");
        assert!(svg.contains(r##"stroke="#ff0000""##), "{svg}");
        assert!(svg.contains(r#"stroke-width="1.5""#), "{svg}");
        // SVG backend emits <path>, never <polyline>.
        assert!(!svg.contains("<polyline"), "{svg}");
    }

    #[test]
    fn path_attribute_values_match_svg_backend() {
        let svg = svg_of(Primitive::Path(Box::new(crate::render::render::PathData {
            d: "M 0 0 L 10 10 Z".to_string(),
            fill: Some(Color::Rgb(16, 32, 48)),
            stroke: Color::Rgb(0, 0, 0),
            stroke_width: 2.0,
            opacity: Some(0.75),
            stroke_dasharray: Some("1 1".to_string()),
        })));
        assert!(svg.contains(r#"d="M 0 0 L 10 10 Z""#), "{svg}");
        assert!(svg.contains(r##"fill="#102030""##), "{svg}");
        assert!(svg.contains(r##"stroke="#000000""##), "{svg}");
        assert!(svg.contains(r#"stroke-width="2""#), "{svg}");
        assert!(svg.contains(r#"fill-opacity="0.75""#), "{svg}");
        assert!(svg.contains(r#"stroke-dasharray="1 1""#), "{svg}");
    }

    #[test]
    fn path_without_fill_emits_fill_none() {
        let svg = svg_of(Primitive::Path(Box::new(crate::render::render::PathData {
            d: "M 0 0 L 1 1".to_string(),
            fill: None,
            stroke: Color::Rgb(0, 0, 0),
            stroke_width: 1.0,
            opacity: None,
            stroke_dasharray: None,
        })));
        assert!(svg.contains(r#"fill="none""#), "{svg}");
        assert!(!svg.contains("fill-opacity="), "{svg}");
    }

    #[test]
    fn text_content_lowers_math_like_the_svg_backend() {
        // `$\sigma^2$` lowers to `σ²`; both backends call the same math helper,
        // so the lowered content must appear verbatim in the SVG output.
        let raw = r"$\sigma^2$";
        let lowered = text_content(raw);
        assert_ne!(lowered, raw, "expected math lowering to rewrite the string");

        let svg = svg_of(Primitive::Text {
            x: 0.0,
            y: 0.0,
            content: raw.to_string(),
            size: 12,
            anchor: TextAnchor::Start,
            rotate: None,
            bold: false,
            color: None,
        });
        assert!(svg.contains(&lowered), "{svg}");
    }

    // ── RichText: <text> + per-span <tspan> styling ──────────────────────────

    #[test]
    fn richtext_styled_and_plain_spans_match_svg_backend() {
        let bold = TextSpan {
            text: "B".to_string(),
            bold: true,
            italic: false,
            underline: false,
            code: false,
        };
        let italic_under = TextSpan {
            text: "iu".to_string(),
            bold: false,
            italic: true,
            underline: true,
            code: false,
        };
        let code = TextSpan {
            text: "c".to_string(),
            bold: false,
            italic: false,
            underline: false,
            code: true,
        };
        let svg = svg_of(Primitive::RichText {
            x: 3.0,
            y: 8.5,
            spans: vec![TextSpan::plain("plain "), bold, italic_under, code],
            size: 11,
            anchor: TextAnchor::End,
            color: Some(Color::Rgb(0, 0, 0)),
        });
        // Root <text> attributes.
        assert!(svg.contains(r#"x="3""#), "{svg}");
        assert!(svg.contains(r#"y="8.5""#), "{svg}");
        assert!(svg.contains(r#"font-size="11""#), "{svg}");
        assert!(svg.contains(r#"text-anchor="end""#), "{svg}");
        assert!(svg.contains(r##"fill="#000000""##), "{svg}");
        // Plain span is bare text (no wrapping tspan around "plain ").
        assert!(svg.contains(">plain "), "{svg}");
        // Styled spans become <tspan> with the SVG backend's attr set.
        assert!(
            svg.contains(r#"<tspan font-weight="bold">B</tspan>"#),
            "{svg}"
        );
        assert!(
            svg.contains(r#"<tspan font-style="italic" text-decoration="underline">iu</tspan>"#),
            "{svg}"
        );
        assert!(
            svg.contains(r#"<tspan font-family="DejaVu Sans Mono,monospace">c</tspan>"#),
            "{svg}"
        );
    }

    // ── CircleBatch: shared fill/r, one <circle> per index ───────────────────

    #[test]
    fn circle_batch_emits_one_circle_per_index_with_shared_attrs() {
        let svg = svg_of(Primitive::CircleBatch {
            cx: vec![1.0, 2.5, 4.0],
            cy: vec![10.0, 20.0, 30.0],
            r: 3.0,
            fill: Color::Rgb(255, 0, 0),
            fill_opacity: Some(0.6),
            stroke: Some(Color::Rgb(0, 0, 0)),
            stroke_width: Some(0.5),
        });
        // Three circles, each with its own center, sharing r/fill/opacity/stroke.
        assert_eq!(svg.matches("<circle").count(), 3, "{svg}");
        assert!(svg.contains(r#"cx="1""#), "{svg}");
        assert!(svg.contains(r#"cx="2.5""#), "{svg}");
        assert!(svg.contains(r#"cx="4""#), "{svg}");
        assert!(svg.contains(r#"cy="30""#), "{svg}");
        assert!(svg.contains(r#"r="3""#), "{svg}");
        assert!(svg.contains(r##"fill="#ff0000""##), "{svg}");
        assert!(svg.contains(r#"fill-opacity="0.6""#), "{svg}");
        assert!(svg.contains(r##"stroke="#000000""##), "{svg}");
        assert!(svg.contains(r#"stroke-width="0.5""#), "{svg}");
    }

    // ── RectBatch: per-rect fill, one <rect> per index ───────────────────────

    #[test]
    fn rect_batch_emits_one_rect_per_index_with_own_fill() {
        let svg = svg_of(Primitive::RectBatch {
            x: vec![0.0, 5.0],
            y: vec![1.0, 6.0],
            w: vec![4.0, 4.0],
            h: vec![2.0, 2.5],
            fills: vec![Color::Rgb(16, 32, 48), Color::Rgb(255, 255, 255)],
        });
        // Two batch rects share width=4 (the full-bleed background rect uses
        // width="100%", so counting on the batch width isolates the batch).
        assert_eq!(svg.matches(r#"width="4""#).count(), 2, "{svg}");
        assert!(svg.contains(r#"x="5""#), "{svg}");
        assert!(svg.contains(r#"height="2.5""#), "{svg}");
        // Each rect carries its own fill.
        assert!(svg.contains(r##"fill="#102030""##), "{svg}");
        assert!(svg.contains(r##"fill="#ffffff""##), "{svg}");
    }

    // ── Flat -> nested fold (the one piece of novel logic) ────────────────────
    //
    // `Element` is opaque, so these assert on the introspectable `Node` tree that
    // `fold_scene` produces, covering nesting, clip wrappers, the close-marker
    // interchangeability `svg.rs` relies on, and the degenerate/unbalanced cases.

    /// A trivial leaf primitive whose `cx` doubles as an identity tag in tests.
    fn dot(tag: f64) -> Primitive {
        Primitive::Circle {
            cx: tag,
            cy: 0.0,
            r: 1.0,
            fill: Color::Rgb(0, 0, 0),
            fill_opacity: None,
            stroke: None,
            stroke_width: None,
        }
    }

    /// The `cx` of a `Node::Leaf(Circle)` built by [`dot`], for identity asserts.
    fn leaf_tag(node: &Node) -> f64 {
        match node {
            Node::Leaf(Primitive::Circle { cx, .. }) => *cx,
            _ => panic!("expected a leaf dot"),
        }
    }

    fn group(transform: Option<&str>, title: Option<&str>, extra: Option<&str>) -> Primitive {
        Primitive::GroupStart {
            transform: transform.map(str::to_string),
            title: title.map(str::to_string),
            extra_attrs: extra.map(str::to_string),
        }
    }

    fn clip(id: &str) -> Primitive {
        Primitive::ClipStart {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
            id: id.to_string(),
        }
    }

    #[test]
    fn fold_wraps_balanced_group_around_its_children() {
        let elems = vec![
            dot(1.0),
            group(None, None, None),
            dot(2.0),
            dot(3.0),
            Primitive::GroupEnd,
            dot(4.0),
        ];
        let tree = fold_scene(&elems);
        assert_eq!(tree.len(), 3); // dot, group, dot
        assert_eq!(leaf_tag(&tree[0]), 1.0);
        assert_eq!(leaf_tag(&tree[2]), 4.0);
        match &tree[1] {
            Node::Group { children, .. } => {
                assert_eq!(children.len(), 2);
                assert_eq!(leaf_tag(&children[0]), 2.0);
                assert_eq!(leaf_tag(&children[1]), 3.0);
            }
            _ => panic!("expected a group at index 1"),
        }
    }

    #[test]
    fn fold_nests_groups_and_clips() {
        let elems = vec![
            group(Some("translate(1,2)"), None, None),
            clip("c0"),
            dot(9.0),
            Primitive::ClipEnd,
            Primitive::GroupEnd,
        ];
        let tree = fold_scene(&elems);
        assert_eq!(tree.len(), 1);
        match &tree[0] {
            Node::Group {
                transform,
                children,
                ..
            } => {
                assert_eq!(transform.as_deref(), Some("translate(1,2)"));
                assert_eq!(children.len(), 1);
                match &children[0] {
                    Node::Clip { id, children } => {
                        assert_eq!(*id, "c0");
                        assert_eq!(children.len(), 1);
                        assert_eq!(leaf_tag(&children[0]), 9.0);
                    }
                    _ => panic!("expected a clip inside the group"),
                }
            }
            _ => panic!("expected an outer group"),
        }
    }

    #[test]
    fn fold_treats_close_markers_interchangeably_like_svg() {
        // `svg.rs` pops one nesting level on EITHER GroupEnd or ClipEnd (both emit
        // `</g>`). A `ClipEnd` therefore closes a group opened by `GroupStart`.
        let elems = vec![
            group(None, None, None),
            dot(5.0),
            Primitive::ClipEnd,
            dot(6.0),
        ];
        let tree = fold_scene(&elems);
        assert_eq!(tree.len(), 2);
        match &tree[0] {
            Node::Group { children, .. } => {
                assert_eq!(children.len(), 1);
                assert_eq!(leaf_tag(&children[0]), 5.0);
            }
            _ => panic!("expected a group closed by the ClipEnd"),
        }
        assert_eq!(leaf_tag(&tree[1]), 6.0); // 6.0 is back at top level
    }

    #[test]
    fn fold_unbalanced_open_folds_to_end_without_panic() {
        // GroupStart with no matching close: its children are everything after it.
        let elems = vec![dot(1.0), group(None, None, None), dot(2.0), dot(3.0)];
        let tree = fold_scene(&elems);
        assert_eq!(tree.len(), 2);
        assert_eq!(leaf_tag(&tree[0]), 1.0);
        match &tree[1] {
            Node::Group { children, .. } => {
                assert_eq!(children.len(), 2);
                assert_eq!(leaf_tag(&children[0]), 2.0);
                assert_eq!(leaf_tag(&children[1]), 3.0);
            }
            _ => panic!("expected the unterminated group"),
        }
    }

    #[test]
    fn fold_stray_close_at_top_level_is_skipped() {
        let elems = vec![dot(1.0), Primitive::GroupEnd, dot(2.0), Primitive::ClipEnd];
        let tree = fold_scene(&elems);
        // Both stray closes vanish; the two leaves survive at top level.
        assert_eq!(tree.len(), 2);
        assert_eq!(leaf_tag(&tree[0]), 1.0);
        assert_eq!(leaf_tag(&tree[1]), 2.0);
    }

    #[test]
    fn fold_empty_scene_is_empty() {
        assert!(fold_scene(&[]).is_empty());
    }

    // ── Group class resolution + extra_attrs parsing ──────────────────────────

    #[test]
    fn extract_attr_value_parses_and_misses() {
        let blob = r#"class="tt" data-group="north 2""#;
        assert_eq!(extract_attr_value(blob, "class").as_deref(), Some("tt"));
        assert_eq!(
            extract_attr_value(blob, "data-group").as_deref(),
            Some("north 2")
        );
        assert_eq!(extract_attr_value(blob, "missing"), None);
        // Unterminated value -> None, not a panic.
        assert_eq!(extract_attr_value(r#"class="oops"#, "class"), None);
    }

    #[test]
    fn group_class_follows_svg_precedence() {
        // class= inside extra_attrs wins outright.
        assert_eq!(
            group_class(
                &Some("t".into()),
                &Some(r#"class="legend" data-group="x""#.into())
            ),
            Some("legend".to_string())
        );
        // No class in extra_attrs but a title -> "tt".
        assert_eq!(
            group_class(&Some("t".into()), &Some(r#"data-group="x""#.into())),
            Some("tt".to_string())
        );
        // Title only, no extra_attrs -> "tt".
        assert_eq!(
            group_class(&Some("t".into()), &None),
            Some("tt".to_string())
        );
        // Neither -> no class.
        assert_eq!(group_class(&None, &None), None);
        // extra_attrs without class and no title -> no class.
        assert_eq!(group_class(&None, &Some(r#"data-group="x""#.into())), None);
    }

    // ── Native hover tooltip gating ───────────────────────────────────────────
    //
    // The rendered `<g>` (with its event handlers) is opaque, so assert on the
    // pure helper that decides the interactive-vs-static branch and the tooltip
    // text, plus the `Tooltip` value semantics the signal/overlay rely on.

    #[test]
    fn group_tooltip_text_is_the_title_only_when_enabled() {
        let titled = Some("north 2".to_string());
        // Enabled + titled -> that title is the hover text.
        assert_eq!(group_tooltip_text(&titled, true), Some("north 2"));
        // Disabled (static path) -> None, so the static `<title>` form is used.
        assert_eq!(group_tooltip_text(&titled, false), None);
        // Enabled but untitled -> nothing to show (mirrors svg.rs's has_tooltips
        // gate: only `GroupStart { title: Some(_) }` participates).
        assert_eq!(group_tooltip_text(&None, true), None);
        assert_eq!(group_tooltip_text(&None, false), None);
    }

    #[test]
    fn tooltip_value_equality_drives_signal_memoization() {
        // Option<Tooltip> rides a Signal/ReadSignal that memoizes by value, so
        // equal states must compare equal and a position/text/pin change must not.
        let a = Tooltip {
            text: "x".into(),
            x: 1.0,
            y: 2.0,
            pinned: false,
        };
        let b = a.clone();
        assert_eq!(a, b);
        assert_ne!(
            a,
            Tooltip {
                text: "x".into(),
                x: 1.0,
                y: 3.0,
                pinned: false,
            }
        );
        assert_ne!(
            a,
            Tooltip {
                text: "y".into(),
                x: 1.0,
                y: 2.0,
                pinned: false,
            }
        );
        // Pinning is part of the value, so it too drives memoization (the overlay
        // must re-render when a hovered tooltip becomes pinned).
        assert_ne!(
            a,
            Tooltip {
                text: "x".into(),
                x: 1.0,
                y: 2.0,
                pinned: true,
            }
        );
    }

    // ── Click-to-pin: toggle + hover-freeze gate ──────────────────────────────
    //
    // The rendered `<g>` (with its `onclick`/hover handlers) is opaque, so assert
    // on the pure helpers the handlers delegate to: `pin_on_click` (the click
    // toggle) and `hover_locked` (whether hover must leave a pinned tooltip alone).

    fn tip(text: &str, x: f64, y: f64, pinned: bool) -> Tooltip {
        Tooltip {
            text: text.into(),
            x,
            y,
            pinned,
        }
    }

    #[test]
    fn pin_on_click_pins_then_unpins_the_same_group() {
        // First click (nothing shown) -> pin this group's tooltip at the click pos.
        let first = pin_on_click(&None, "Mon", 12.0, 34.0);
        assert_eq!(first, Some(tip("Mon", 12.0, 34.0, true)));
        // Clicking the already-pinned same group -> unpin (clear the overlay).
        assert_eq!(pin_on_click(&first, "Mon", 99.0, 99.0), None);
    }

    #[test]
    fn pin_on_click_repins_from_hover_or_a_different_group() {
        // A click while a tooltip is merely *hovered* (unpinned) pins it.
        let hovered = Some(tip("Mon", 1.0, 2.0, false));
        assert_eq!(
            pin_on_click(&hovered, "Mon", 5.0, 6.0),
            Some(tip("Mon", 5.0, 6.0, true))
        );
        // A click on a DIFFERENT group while one is pinned re-pins to the new group
        // (the same-text unpin guard doesn't fire).
        let pinned_mon = Some(tip("Mon", 1.0, 2.0, true));
        assert_eq!(
            pin_on_click(&pinned_mon, "Tue", 7.0, 8.0),
            Some(tip("Tue", 7.0, 8.0, true))
        );
    }

    #[test]
    fn hover_locked_only_when_a_pinned_tooltip_is_shown() {
        assert!(!hover_locked(&None));
        assert!(!hover_locked(&Some(tip("Mon", 0.0, 0.0, false))));
        assert!(hover_locked(&Some(tip("Mon", 0.0, 0.0, true))));
    }

    // ── Legend toggle: series filtering + legend-entry classification (4.2) ────
    //
    // The rendered `<g>` (filtered out, or carrying the toggle `onclick`) is opaque,
    // so assert on the pure helpers that classify a group from its `extra_attrs` and
    // decide whether a series is dropped. The literal attr blobs below are the exact
    // format kuva emits in interactive mode (`add_line` for the series group, the
    // legend `render_entry` for the swatch); the live test at the end pins that
    // format against the real interactive pipeline.

    /// A data-series group as `add_line` emits it in interactive mode.
    fn series_attrs(label: &str) -> Option<String> {
        Some(format!(r#"class="tt" data-group="{label}""#))
    }
    /// A legend-entry group as the legend `render_entry` emits it in interactive mode.
    fn legend_attrs(label: &str) -> Option<String> {
        Some(format!(r#"class="legend-entry" data-group="{label}""#))
    }

    #[test]
    fn group_data_group_extracts_the_series_label() {
        // Both the data series and its legend swatch carry the same data-group id.
        assert_eq!(
            group_data_group(&series_attrs("Series A")).as_deref(),
            Some("Series A")
        );
        assert_eq!(
            group_data_group(&legend_attrs("Series A")).as_deref(),
            Some("Series A")
        );
        // No data-group / no extra_attrs -> None.
        assert_eq!(group_data_group(&Some(r#"class="tt""#.into())), None);
        assert_eq!(group_data_group(&None), None);
    }

    #[test]
    fn is_legend_entry_distinguishes_legend_from_series() {
        assert!(is_legend_entry(&legend_attrs("X")));
        assert!(!is_legend_entry(&series_attrs("X")));
        assert!(!is_legend_entry(&None));
    }

    #[test]
    fn series_is_hidden_drops_only_hidden_data_series() {
        let mut hidden = HashSet::new();
        hidden.insert("Series A".to_string());
        // A data series whose label is hidden -> dropped from the render.
        assert!(series_is_hidden(&series_attrs("Series A"), &hidden));
        // A data series not in the set -> kept.
        assert!(!series_is_hidden(&series_attrs("Series B"), &hidden));
        // The legend entry for the hidden series is NEVER dropped (so the series can
        // be toggled back on) — even though it carries the same data-group id.
        assert!(!series_is_hidden(&legend_attrs("Series A"), &hidden));
        // A group with no data-group is never filtered.
        assert!(!series_is_hidden(&Some(r#"class="tt""#.into()), &hidden));
        assert!(!series_is_hidden(&None, &hidden));
    }

    #[test]
    fn interactive_line_plot_tags_series_and_legend_with_data_group() {
        // The whole legend toggle keys off `data-group`, present only in interactive
        // mode. Prove the live pipeline emits BOTH group kinds our helpers parse —
        // the data series (`class="tt" data-group=`) and its legend swatch
        // (`class="legend-entry" data-group=`) — for a labelled line series, so the
        // literal blobs the tests above use match what kuva actually produces.
        use crate::plot::line::LinePlot;
        use crate::render::render::collect_legend_entries;

        let plots = vec![Plot::Line(
            LinePlot::new()
                .with_data(vec![(0.0, 1.0), (1.0, 2.0)])
                .with_legend("Series A"),
        )];
        let entries = collect_legend_entries(&plots);
        let layout = Layout::auto_from_plots(&plots)
            .with_interactive()
            .with_legend_entries(entries);
        let scene = scene_from_plots(plots, Some(layout));
        let svg = SvgBackend::new().render_scene(&scene);

        assert!(
            svg.contains(r#"class="tt" data-group="Series A""#),
            "expected an interactive data-series group: {svg}"
        );
        assert!(
            svg.contains(r#"class="legend-entry" data-group="Series A""#),
            "expected an interactive legend-entry group: {svg}"
        );
    }

    // ── Coordinate readout: pixel→data conversion + formatting ─────────────────
    //
    // The rendered `<svg>` + its mousemove handler are opaque, so assert on the
    // pure conversion (`px_to_data`), the value formatting (`fmt_readout`), and the
    // bounds-gated readout string (`readout_text`) — the JS-mirroring core the
    // handler delegates to. A live test pins that an interactive axis plot actually
    // produces the `axis_meta` the readout depends on.

    /// Linear axes: plot rect x∈[10,110]→data x∈[0,100], y∈[10,210]→data y∈[0,50]
    /// (y flipped: screen-top = data-max).
    fn linear_axes() -> Axes {
        Axes {
            x_min: 0.0,
            x_max: 100.0,
            y_min: 0.0,
            y_max: 50.0,
            plot_left: 10.0,
            plot_top: 10.0,
            plot_right: 110.0,
            plot_bottom: 210.0,
            log_x: false,
            log_y: false,
        }
    }

    #[test]
    fn px_to_data_mirrors_the_js_linear_mapping() {
        let a = linear_axes();
        // Plot-rect corners map to the data-range corners (note the y-flip).
        assert_eq!(px_to_data(a, 10.0, 210.0), (0.0, 0.0)); // bottom-left
        assert_eq!(px_to_data(a, 110.0, 10.0), (100.0, 50.0)); // top-right
                                                               // Midpoints interpolate linearly.
        let (x, y) = px_to_data(a, 60.0, 110.0);
        assert!((x - 50.0).abs() < 1e-9, "x={x}");
        assert!((y - 25.0).abs() < 1e-9, "y={y}");
    }

    #[test]
    fn px_to_data_handles_log_axes() {
        // log_x: data x = 10^(x_min + fx*(x_max-x_min)). x_min=0, x_max=3 → decades.
        let a = Axes {
            x_min: 0.0,
            x_max: 3.0,
            log_x: true,
            ..linear_axes()
        };
        let (x0, _) = px_to_data(a, 10.0, 110.0); // fx=0 → 10^0 = 1
        assert!((x0 - 1.0).abs() < 1e-9, "x0={x0}");
        let (x1, _) = px_to_data(a, 110.0, 110.0); // fx=1 → 10^3 = 1000
        assert!((x1 - 1000.0).abs() < 1e-6, "x1={x1}");
        let (xm, _) = px_to_data(a, 60.0, 110.0); // fx=0.5 → 10^1.5 ≈ 31.6228
        assert!((xm - 31.622_776_6).abs() < 1e-5, "xm={xm}");
    }

    #[test]
    fn fmt_readout_matches_js_semantics() {
        // Mid-range: 3dp with trailing zeros/dot trimmed.
        assert_eq!(fmt_readout(50.0), "50");
        assert_eq!(fmt_readout(1.5), "1.5");
        assert_eq!(fmt_readout(1.234), "1.234");
        assert_eq!(fmt_readout(0.0), "0");
        assert_eq!(fmt_readout(0.01), "0.01"); // boundary: |v| not < 0.01 → mid-range
                                               // Large/tiny magnitudes: exponential (2dp). Rust omits the `+` JS would add
                                               // in a positive exponent — an intentional, irrelevant divergence.
        assert_eq!(fmt_readout(12345.0), "1.23e4");
        assert_eq!(fmt_readout(0.005), "5.00e-3");
    }

    #[test]
    fn readout_text_is_gated_by_the_plot_bounds() {
        let a = linear_axes();
        // Inside the plot rect → "x=…  y=…" (two spaces, matching the JS).
        assert_eq!(readout_text(a, 60.0, 110.0).as_deref(), Some("x=50  y=25"));
        assert_eq!(readout_text(a, 110.0, 10.0).as_deref(), Some("x=100  y=50"));
        // Outside any edge → None (the JS clears the readout there).
        assert_eq!(readout_text(a, 5.0, 110.0), None); // left of plot_left
        assert_eq!(readout_text(a, 60.0, 5.0), None); // above plot_top
        assert_eq!(readout_text(a, 200.0, 110.0), None); // right of plot_right
        assert_eq!(readout_text(a, 60.0, 300.0), None); // below plot_bottom
    }

    #[test]
    fn axes_from_meta_copies_the_axis_metadata() {
        let meta = AxisMeta {
            x_min: 1.0,
            x_max: 2.0,
            y_min: 3.0,
            y_max: 4.0,
            plot_left: 5.0,
            plot_top: 6.0,
            plot_right: 7.0,
            plot_bottom: 8.0,
            log_x: true,
            log_y: false,
        };
        let a = Axes::from_meta(&meta);
        assert_eq!(
            a,
            Axes {
                x_min: 1.0,
                x_max: 2.0,
                y_min: 3.0,
                y_max: 4.0,
                plot_left: 5.0,
                plot_top: 6.0,
                plot_right: 7.0,
                plot_bottom: 8.0,
                log_x: true,
                log_y: false,
            }
        );
    }

    #[test]
    fn interactive_axis_plot_produces_axis_meta_the_readout_consumes() {
        // The readout keys off `Scene.axis_meta`, populated only for interactive
        // axis plots (not pixel-space ones). Prove the live pipeline emits it and
        // that a cursor at the plot center yields a readout string through the same
        // `Axes`/`readout_text` path the handler uses.
        use crate::plot::scatter::ScatterPlot;

        let plots = vec![Plot::Scatter(
            ScatterPlot::new().with_data(vec![(0.0, 0.0), (10.0, 10.0)]),
        )];
        let layout = Layout::auto_from_plots(&plots).with_interactive();
        let scene = scene_from_plots(plots, Some(layout));

        let meta = scene
            .axis_meta
            .as_ref()
            .expect("an interactive scatter plot must carry axis_meta");
        let a = Axes::from_meta(meta);
        // The plot-rect center is in-bounds → a readout string is produced.
        let cx = (a.plot_left + a.plot_right) / 2.0;
        let cy = (a.plot_top + a.plot_bottom) / 2.0;
        let text = readout_text(a, cx, cy).expect("center is inside the plot bounds");
        assert!(text.starts_with("x="), "{text}");
        assert!(text.contains("  y="), "{text}");
    }

    // ── Text annotations / value labels lower to mapped leaf primitives ────────

    #[test]
    fn text_annotation_lowers_to_leaf_primitives_the_backend_maps() {
        // A value label is just a Layout `TextAnnotation` (kuva's documented
        // mechanism — there is no per-bar/per-point value-label builder). The
        // pipeline lowers it to plain `Text` (+ `Line` shaft and `Path` arrowhead
        // for `.with_arrow`) *leaf* primitives — all three already mapped and
        // parity-tested by this backend (Phases 1.0–1.1). So value labels render
        // natively with NO annotation-specific backend code; this test pins that the
        // pipeline actually surfaces those primitives.
        use crate::render::annotations::TextAnnotation;

        let plots = vec![scatter()];
        let layout = Layout::auto_from_plots(&plots).with_annotation(
            TextAnnotation::new("peak", 2.0, 3.0)
                .with_arrow(2.5, 2.5)
                .with_color("crimson"),
        );
        let scene = scene_from_plots(plots, Some(layout));

        // The annotation produced a Text leaf carrying our label...
        assert!(
            scene
                .elements
                .iter()
                .any(|e| matches!(e, Primitive::Text { content, .. } if content == "peak")),
            "annotation label text not surfaced into the Scene"
        );
        // ...plus the arrow shaft (Line) and arrowhead (Path).
        assert!(
            scene
                .elements
                .iter()
                .any(|e| matches!(e, Primitive::Line { .. })),
            "annotation arrow line not surfaced"
        );
        assert!(
            scene
                .elements
                .iter()
                .any(|e| matches!(e, Primitive::Path(_))),
            "annotation arrowhead path not surfaced"
        );

        // And the live SVG backend renders that label verbatim — our per-variant
        // parity tests guarantee the Dioxus `Text`/`Line`/`Path` mapping matches it.
        let svg = SvgBackend::new().render_scene(&scene);
        assert!(
            svg.contains(">peak<"),
            "annotation label not rendered: {svg}"
        );
    }

    // ── defs / scripts dangerous_inner_html islands ───────────────────────────

    #[test]
    fn defs_inner_html_is_none_when_empty_and_concats_otherwise() {
        let mut scene = Scene::new(10.0, 10.0);
        assert_eq!(defs_inner_html(&scene), None);

        // svg.rs writes each defs entry verbatim with no separator; mirror that.
        scene.defs.push(r#"<linearGradient id="g0"/>"#.to_string());
        scene
            .defs
            .push(r#"<clipPath id="c0"><rect/></clipPath>"#.to_string());
        assert_eq!(
            defs_inner_html(&scene).as_deref(),
            Some(r#"<linearGradient id="g0"/><clipPath id="c0"><rect/></clipPath>"#)
        );
    }

    #[test]
    fn defs_island_matches_svg_backend_defs_block() {
        // The raw def the island carries must appear verbatim in the live SVG
        // backend's <defs> block, so the two serializers agree on the defs.
        let mut scene = Scene::new(100.0, 100.0);
        let def = r#"<linearGradient id="grad0"><stop offset="0%"/></linearGradient>"#;
        scene.defs.push(def.to_string());
        let svg = SvgBackend::new().render_scene(&scene);
        assert!(svg.contains("<defs>"), "{svg}");
        assert!(svg.contains(def), "{svg}");
        // Our island carries exactly that string.
        assert_eq!(defs_inner_html(&scene).as_deref(), Some(def));
    }

    // ── scene_from_plots: the pure half of the KuvaPlot wrapper ───────────────
    //
    // The component itself needs a VDOM/renderer to exercise (out of scope for
    // the host test convention), but its load-bearing logic — running kuva's
    // `plot -> Scene` pipeline and choosing the layout — is a pure function we
    // can assert on directly.

    use crate::plot::scatter::ScatterPlot;

    fn scatter() -> Plot {
        Plot::Scatter(ScatterPlot::new().with_data(vec![(1.0, 2.0), (2.0, 3.0), (3.0, 1.0)]))
    }

    #[test]
    fn scene_from_plots_honours_an_explicit_layout_size() {
        let layout = Layout::auto_from_plots(&[scatter()])
            .with_width(640.0)
            .with_height(400.0);
        let scene = scene_from_plots(vec![scatter()], Some(layout));
        // The explicit Layout dimensions are baked into the Scene verbatim.
        assert_eq!(scene.width, 640.0);
        assert_eq!(scene.height, 400.0);
        // The pipeline actually drew something (axes, points, …).
        assert!(!scene.elements.is_empty());
    }

    #[test]
    fn scene_from_plots_falls_back_to_auto_layout() {
        // No layout supplied -> auto_from_plots is used; the resulting Scene must
        // match what an explicit auto layout produces (same dimensions, drawn).
        let auto = scene_from_plots(vec![scatter()], None);
        let explicit =
            scene_from_plots(vec![scatter()], Some(Layout::auto_from_plots(&[scatter()])));
        assert_eq!(auto.width, explicit.width);
        assert_eq!(auto.height, explicit.height);
        assert!(auto.width > 0.0 && auto.height > 0.0);
        assert!(!auto.elements.is_empty());
    }

    #[test]
    fn scene_from_plots_matches_the_svg_backend_pipeline() {
        // KuvaPlot must tap the *same* seam SvgBackend consumers tap: identical
        // plots + layout -> identical Scene. Prove it by feeding the Scene this
        // helper builds to the SVG backend and confirming it renders a chart.
        let scene = scene_from_plots(vec![scatter()], None);
        let svg = SvgBackend::new().render_scene(&scene);
        assert!(svg.starts_with("<svg"), "{svg}");
        assert!(svg.contains("<circle"), "expected scatter points: {svg}");
    }

    /// Replace every `kuva-clip-<n>` id with a fixed `kuva-clip-N` so two renders
    /// can be compared for *structural* equality. kuva mints clip-path ids from a
    /// process-global monotonic counter (so the Nth `clipPath` in a process gets a
    /// different number on each call), but the `<clipPath id>` def and the
    /// `url(#id)` wrapper that references it always move together — the render
    /// stays internally consistent and visually identical, only the id digits
    /// differ. The Scene is otherwise a pure function of its data.
    fn normalize_clip_ids(s: &str) -> String {
        let mut parts = s.split("kuva-clip-");
        let mut out = parts.next().unwrap_or("").to_string();
        for part in parts {
            out.push_str("kuva-clip-N");
            out.push_str(part.trim_start_matches(|c: char| c.is_ascii_digit()));
        }
        out
    }

    #[test]
    fn scene_from_plots_is_a_pure_function_of_its_data() {
        // The reactive keying — `KuvaPlot` rebuilds the Scene only when
        // its `data` signal changes — is only sound if the Scene is a
        // deterministic function of that data: same data must give the same
        // Scene, different data a different one. Pin both directions, modulo the
        // global clip-id counter (see `normalize_clip_ids`). (`Scene` has no
        // `PartialEq`, so compare via the SvgBackend serialization.)
        let render = |pts: Vec<(f64, f64)>| {
            let plot = Plot::Scatter(ScatterPlot::new().with_data(pts));
            normalize_clip_ids(&SvgBackend::new().render_scene(&scene_from_plots(vec![plot], None)))
        };
        let a1 = render(vec![(1.0, 2.0), (2.0, 3.0), (3.0, 1.0)]);
        let a2 = render(vec![(1.0, 2.0), (2.0, 3.0), (3.0, 1.0)]);
        let b = render(vec![(1.0, 9.0), (2.0, 8.0), (3.0, 7.0)]);
        assert_eq!(
            a1, a2,
            "same data must yield a structurally identical Scene"
        );
        assert_ne!(a1, b, "different data must yield a different Scene");
    }
}
