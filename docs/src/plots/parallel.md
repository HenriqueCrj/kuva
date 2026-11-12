# Parallel Coordinates Plot

A parallel coordinates plot displays multivariate data by drawing one vertical axis per dimension and connecting each observation as a polyline that passes through its value on each axis. Groups of observations that share a similar pattern appear as bundles of lines with similar trajectories; divergent groups cross each other clearly.

Parallel coordinates are useful for exploring high-dimensional datasets, comparing groups across many measured attributes, and identifying which dimensions best separate groups.

**Import path:** `kuva::plot::parallel::{ParallelPlot, ParallelRow}`

---

## Basic usage

Set axis names with `.with_axis_names()`, then add rows with `.with_row_group(group, values)`. Each row is one observation; values must be in the same order as the axis names.

```rust,no_run
use kuva::plot::parallel::ParallelPlot;
use kuva::backend::svg::SvgBackend;
use kuva::render::render::render_multiple;
use kuva::render::layout::Layout;
use kuva::render::plots::Plot;

let plot = ParallelPlot::new()
    .with_axis_names(["Sepal L", "Sepal W", "Petal L", "Petal W"])
    .with_row_group("setosa",     vec![5.1, 3.5, 1.4, 0.2])
    .with_row_group("setosa",     vec![4.9, 3.0, 1.4, 0.2])
    .with_row_group("setosa",     vec![4.7, 3.2, 1.3, 0.2])
    .with_row_group("versicolor", vec![7.0, 3.2, 4.7, 1.4])
    .with_row_group("versicolor", vec![6.4, 3.2, 4.5, 1.5])
    .with_row_group("versicolor", vec![6.9, 3.1, 4.9, 1.5])
    .with_row_group("virginica",  vec![6.3, 3.3, 6.0, 2.5])
    .with_row_group("virginica",  vec![5.8, 2.7, 5.1, 1.9])
    .with_row_group("virginica",  vec![7.1, 3.0, 5.9, 2.1])
    .with_legend("Species");

let plots = vec![Plot::Parallel(plot)];
let layout = Layout::auto_from_plots(&plots)
    .with_title("Iris Dataset");

let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
std::fs::write("parallel.svg", svg).unwrap();
```

<img src="../assets/parallel/basic.svg" alt="Parallel coordinates plot of the Iris dataset" width="560">

Each axis is normalised to `[0, 1]` by default so that differently-scaled dimensions are comparable. Disable with `.with_normalize(false)` when all axes share a common unit.

---

## Smooth curves

`.with_curved(true)` draws S-shaped cubic Bézier curves instead of straight polylines, which reduces visual clutter in dense plots.

```rust,no_run
use kuva::plot::parallel::ParallelPlot;
use kuva::render::plots::Plot;
# use kuva::render::layout::Layout;
# use kuva::render::render::render_multiple;

let plot = ParallelPlot::new()
    .with_axis_names(["Sepal L", "Sepal W", "Petal L", "Petal W"])
    .with_row_group("setosa",     vec![5.1, 3.5, 1.4, 0.2])
    .with_row_group("setosa",     vec![4.9, 3.0, 1.4, 0.2])
    .with_row_group("versicolor", vec![7.0, 3.2, 4.7, 1.4])
    .with_row_group("versicolor", vec![6.4, 3.2, 4.5, 1.5])
    .with_row_group("virginica",  vec![6.3, 3.3, 6.0, 2.5])
    .with_row_group("virginica",  vec![7.1, 3.0, 5.9, 2.1])
    .with_curved(true)
    .with_opacity(0.7)
    .with_legend("Species");

let plots = vec![Plot::Parallel(plot)];
```

<img src="../assets/parallel/curved.svg" alt="Parallel coordinates with smooth Bézier curves" width="560">

---

## Group mean overlay

`.with_mean(true)` draws a bold polyline at the per-group mean for each axis. This makes the group-level pattern visible even when individual lines are dense.

```rust,no_run
use kuva::plot::parallel::ParallelPlot;
use kuva::render::plots::Plot;
use kuva::render::layout::Layout;
use kuva::render::render::render_multiple;
use kuva::backend::svg::SvgBackend;

let plot = ParallelPlot::new()
    .with_axis_names(["Recall", "Precision", "F1", "AUC", "Inference ms"])
    // Multiple runs per model
    .with_group_rows("BERT",  [
        vec![0.82, 0.85, 0.83, 0.91, 120.0],
        vec![0.80, 0.87, 0.83, 0.90, 115.0],
        vec![0.83, 0.84, 0.83, 0.92, 125.0],
    ])
    .with_group_rows("DistilBERT", [
        vec![0.78, 0.80, 0.79, 0.87,  55.0],
        vec![0.77, 0.82, 0.79, 0.86,  52.0],
        vec![0.79, 0.81, 0.80, 0.88,  58.0],
    ])
    .with_group_rows("LSTM", [
        vec![0.72, 0.75, 0.73, 0.82,  30.0],
        vec![0.71, 0.76, 0.73, 0.81,  28.0],
        vec![0.73, 0.74, 0.73, 0.83,  32.0],
    ])
    .with_mean(true)
    .with_opacity(0.35)
    .with_curved(true)
    .with_legend("Model");

let plots = vec![Plot::Parallel(plot)];
let layout = Layout::auto_from_plots(&plots).with_title("NLP Model Comparison");
let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
```

<img src="../assets/parallel/mean_overlay.svg" alt="Parallel coordinates with group mean overlay" width="560">

---

## Axis inversion

Some axes are naturally "better when low" (e.g., error rate, latency). `.with_inverted_axis(i)` inverts axis `i` so that high values plot near the bottom — a visual triangle at the bottom of the axis label indicates inversion.

```rust,no_run
use kuva::plot::parallel::ParallelPlot;
use kuva::render::plots::Plot;
# use kuva::render::layout::Layout;
# use kuva::render::render::render_multiple;

// Axes: Accuracy (higher = better), Error rate (lower = better), Speed ms (lower = better), F1 (higher = better)
let plot = ParallelPlot::new()
    .with_axis_names(["Accuracy", "Error rate", "Speed (ms)", "F1"])
    .with_row_group("Model A", vec![0.92, 0.08, 120.0, 0.90])
    .with_row_group("Model A", vec![0.91, 0.09, 115.0, 0.89])
    .with_row_group("Model B", vec![0.87, 0.13,  45.0, 0.86])
    .with_row_group("Model B", vec![0.88, 0.12,  48.0, 0.87])
    .with_inverted_axes([1, 2])  // invert Error rate and Speed: down = better
    .with_mean(true)
    .with_legend("Model");

let plots = vec![Plot::Parallel(plot)];
```

---

## ParallelPlot API reference

### `ParallelPlot` builders

| Method | Default | Description |
|--------|---------|-------------|
| `ParallelPlot::new()` | — | Create a parallel coordinates plot |
| `.with_axis_names(iter)` | — | Set axis (column) names |
| `.with_row(values)` | — | Add an ungrouped row |
| `.with_row_group(group, values)` | — | Add a row assigned to a named group |
| `.with_rows(iter)` | — | Add multiple ungrouped rows |
| `.with_group_rows(group, iter)` | — | Add multiple rows to the same group |
| `.with_normalize(bool)` | `true` | Normalise each axis independently to `[0, 1]` |
| `.with_curved(bool)` | `false` | Draw smooth S-shaped Bézier curves |
| `.with_stroke_width(px)` | `1.2` | Polyline stroke width |
| `.with_opacity(f)` | `0.6` | Polyline opacity |
| `.with_color(css)` | `"steelblue"` | Fallback color for ungrouped rows |
| `.with_group_colors(iter)` | palette | Explicit per-group CSS colors |
| `.with_mean(bool)` | `false` | Draw a bold mean line for each group |
| `.with_inverted_axis(i)` | — | Invert axis `i` (high values at bottom) |
| `.with_inverted_axes(iter)` | — | Invert multiple axes |
| `.with_legend(label)` | — | Legend title (one entry per group) |

**See also:** [Radar / Spider Chart](./radar.md) for a polygon-based multivariate view, [Slope Chart](./slope.md) for a simpler two-point comparison.

---

## CLI

Parallel coordinates — multivariate visualisation where each axis represents one variable and each observation is a polyline.

**Input:** one row per observation; `--value-cols` selects the numeric axes; an optional group column colors the lines.

| Flag | Default | Description |
|---|---|---|
| `--value-cols <COL>…` | required | Two or more numeric columns (names or indices) |
| `--group-col <COL>` | — | Group/color column |
| `--axis-names <NAME>…` | header or "Axis N" | Override axis labels |
| `--no-normalize` | off | Disable per-axis normalization to [0, 1] |
| `--curved` | off | Render smooth Bézier curves instead of polylines |
| `--opacity <F>` | `0.5` | Line opacity |
| `--show-mean` | off | Overlay a bold group-mean line |
| `--legend <LABEL>` | — | Add a legend |

```bash
kuva parallel data.tsv \
    --value-cols sepal_length sepal_width petal_length petal_width \
    --group-col species

kuva parallel data.tsv \
    --value-cols 0 1 2 3 --group-col 4 \
    --curved --show-mean --legend "Species" \
    --title "Iris Parallel Coordinates"
```

---

*See also: [Shared flags](../cli/index.md#shared-flags) — output, appearance, axes.*
