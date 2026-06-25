# Dioxus Backend

The `dioxus` feature adds a native [Dioxus](https://dioxuslabs.com/) backend that
serializes a `Scene` to a typed `dioxus::Element` (RSX) tree instead of an SVG
**string**. It is a sibling serializer over `&Scene`, exactly like the SVG and
terminal backends — it implements no shared trait and touches no plot, layout, or
projection code.

This lets a Dioxus 0.7 app render a kuva plot through the real virtual DOM:

- fine-grained diffing — only the nodes that change are re-rendered, instead of
  re-injecting the whole SVG on every data change;
- no `dangerous_inner_html` for the chart body (only the `<defs>`
  gradient/marker/clip-path island, which has no typed-node form);
- signals, i18n strings, and your app's CSS classes are reachable from the
  chart, because it is part of your component tree — not an opaque blob.

It is **off by default**. Enabling it pulls in only `dioxus` (the `macro`, `html`,
and `signals` features — no web/desktop renderer), so the WebAssembly dependency
surface stays minimal and never drags in the native-only deps (`fontdue`, `png`,
`flate2`, `svg2pdf`, `clap`, `arrow`). A CI job guards the
`wasm32-unknown-unknown` build against any such leak.

## Enabling it

```toml
[dependencies]
kuva   = { version = "0.3", features = ["dioxus"] }
# Your app brings its own renderer:
dioxus = { version = "0.7", features = ["web"] }
```

## Quick start

The `KuvaPlot` component takes a reactive `data` signal and a pure
`data -> Scene` `builder`, and rebuilds the `Scene` only when `data` changes:

```rust,ignore
use dioxus::prelude::*;
use kuva::prelude::*;

#[component]
fn Chart() -> Element {
    let points = use_signal(|| vec![(1.0, 2.0), (2.0, 3.0), (3.0, 1.0)]);
    rsx! {
        KuvaPlot {
            data: points,
            // A pure data -> Scene mapping (taps the same seam SvgBackend does).
            builder: move |pts: Vec<(f64, f64)>| scene_from_plots(
                vec![Plot::Scatter(ScatterPlot::new().with_data(pts))],
                Some(Layout::auto_from_plots(&[]).with_width(640.0).with_height(400.0)),
            ),
            // Optional: a CSS class to size the responsive (viewBox-only) chart.
            class: "chart",
        }
    }
}
```

For one-shot rendering without the component, call the backend directly — the
drop-in equivalent of `SvgBackend::render_scene`:

```rust,ignore
let scene = scene_from_plots(plots, None);
let element = KuvaDioxus::new().render_scene(&scene); // -> dioxus::Element
```

## Responsive sizing

Where the SVG backend bakes a fixed `width`/`height` into the root `<svg>`, this
backend emits a `viewBox` (from `Scene.width/height`) and **no** `width`/`height`,
so the chart scales to its container. Attach sizing CSS with
`KuvaDioxus::with_class` or the `KuvaPlot` `class` prop:

```css
.chart { width: 100%; height: auto; }
```

## Theming

By default the backend is at parity with the SVG backend: it emits the
`Scene`'s baked background and text colors. To let a host surface own its
colors instead, remap them:

```rust,ignore
// `Background` / `TextColor` aren't in the prelude (the bare names collide too
// easily with a host app's own types); import them from the crate root.
use kuva::{Background, TextColor};

KuvaDioxus::new()
    .with_background(Background::None)        // host surface shows through
    .with_text_color(TextColor::Token("currentColor".into()));
```

`Background` is one of `Scene` (default), `None`, or `Token(css)`; `TextColor` is
`Scene` (default), `Inherit` (defer to CSS), or `Token(css)`. The same options
are available as `KuvaPlot` props.

## Native interactivity

Instead of porting kuva's injected interactivity JavaScript, the backend rebuilds
the interactions natively from `Scene` metadata, driven by **consumer-owned**
signals (so the backend needs no Dioxus `hooks` feature — you create the
`use_signal` and pass it in):

| Interaction | Prop / method | Notes |
|---|---|---|
| Hover tooltip | `tooltip` | A titled group gets hover handlers; a `KuvaTooltip` overlay shows the text. Replaces the static `<title>`. |
| Click-to-pin | `tooltip` | Clicking a titled group pins its tooltip (frozen until clicked again). Rides the same signal. |
| Legend toggle | `hidden_series` | Click a legend swatch to hide/show its series. Requires `Layout::with_interactive()`. |
| Coordinate readout | `readout` | Moving the cursor over an axis plot shows the data coordinates. Requires `axis_meta` (interactive axis plots). |

The `KuvaTooltip` overlay's appearance is left entirely to the `.kuva-tooltip`
CSS class — the backend bakes in no theme, so the overlay renders **nothing
visible until you add a `.kuva-tooltip` rule**. A reasonable starting point:

```css
.kuva-tooltip {
  background: #222; color: #fff; padding: 2px 6px;
  border-radius: 4px; font-size: 12px; white-space: nowrap;
}
```

The coordinate readout is exact when the chart renders at its intrinsic
(`viewBox`) size; correcting a responsively-scaled chart would require a DOM read
(and thus the Dioxus `hooks` feature this backend deliberately avoids), so it is
not attempted. The SVG backend's search box and save button are not reproduced —
the native path covers hover, pin, legend toggle, and readout.

## Caveats

- **`CalendarPlot`** is the only plot whose interactivity rides `Scene.scripts`
  (injected JS). That JS is dropped (inner-HTML `<script>` never auto-executes in
  the VDOM anyway); use the native interaction hooks above instead.
- Output is **visually** identical to the SVG backend, not byte-identical: RSX
  escapes special characters in text with numeric entities (`&#39;`) where the
  SVG string uses named ones (`&apos;`). The rendered glyphs match.
- Clipped plots mint a fresh `clipPath` id on every `Scene` rebuild (kuva sources
  these from a process-global counter). The `<defs>` island and the matching
  `clip-path="url(#…)"` attributes therefore always count as changed across a
  reactive update, so those few nodes are re-patched even when geometrically
  identical. It is a small attribute patch, not a full re-injection — the
  fine-grained diffing of everything else is unaffected.
