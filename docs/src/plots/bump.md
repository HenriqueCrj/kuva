# Bump Chart

A bump chart shows how the **rank** of each series changes across discrete time points or conditions.  Lines connect consecutive ranks; the best rank (1) appears at the top.

## Basic usage (pre-ranked)

```rust,no_run
use kuva::plot::bump::BumpPlot;
use kuva::render::{plots::Plot, layout::Layout, render::render_multiple};
use kuva::backend::svg::SvgBackend;

let plot = BumpPlot::new()
    .with_series("Alpha", vec![1, 3, 2, 1])
    .with_series("Beta",  vec![2, 1, 1, 3])
    .with_series("Gamma", vec![3, 2, 3, 2])
    .with_x_labels(["2021", "2022", "2023", "2024"]);

let plots = vec![Plot::Bump(plot)];
let layout = Layout::auto_from_plots(&plots).with_title("Rank over time");
let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
std::fs::write("bump.svg", svg).unwrap();
```

## Auto-ranking from raw values

Instead of supplying pre-computed ranks you can provide raw values; kuva ranks them per time point automatically.

```rust,no_run
use kuva::plot::bump::BumpPlot;

let plot = BumpPlot::new()
    .with_raw_series("A", vec![95.0, 80.0, 88.0])
    .with_raw_series("B", vec![80.0, 95.0, 72.0])
    .with_raw_series("C", vec![70.0, 85.0, 95.0])
    .with_x_labels(["Q1", "Q2", "Q3"]);
```

By default, **higher value = rank 1** (better).  Pass `.with_rank_ascending(true)` to flip this so lower value = rank 1.

## Builder reference

| Method | Default | Description |
|--------|---------|-------------|
| `.with_series(name, ranks)` | — | Add a pre-ranked series (integer or float ranks). |
| `.with_ranked_series(name, ranks)` | — | Pre-ranked series that allows `None` gaps. |
| `.with_raw_series(name, values)` | — | Raw values; ranks computed automatically. |
| `.with_raw_series_opt(name, values)` | — | Raw values with optional gaps (`None` breaks the line). |
| `.with_x_labels(labels)` | — | Labels for each time point / condition on the x-axis. |
| `.with_curve_style(style)` | `Sigmoid` | Line style between rank points: `Sigmoid` or `Straight`. |
| `.with_show_rank_labels(bool)` | `false` | Draw the rank number inside each dot. |
| `.with_show_series_labels(bool)` | `true` | Draw series name labels at the left and right edges. |
| `.with_dot_radius(f64)` | `6.0` | Dot radius in pixels. |
| `.with_stroke_width(f64)` | `2.5` | Line stroke width in pixels. |
| `.with_highlight(name)` | `None` | Highlight one series; all others are muted to 20 % opacity. |
| `.with_legend(bool)` | `true` | Show / hide the legend. |
| `.with_rank_ascending(bool)` | `false` | If `true`, lower raw value → better (lower) rank number. |
| `.with_tie_break(mode)` | `Average` | Tie-breaking for auto-ranking: `Average`, `Min`, `Max`, `Stable`. |

## Highlight mode

Highlighting one series draws it with a thicker stroke and bolder endpoint labels; all others are rendered at reduced opacity and with muted grey labels.

```rust,no_run
let plot = BumpPlot::new()
    .with_series("Alpha", vec![1, 3, 2, 1])
    .with_series("Beta",  vec![2, 1, 1, 3])
    .with_series("Gamma", vec![3, 2, 3, 2])
    .with_highlight("Alpha");
```

## Missing time points

Supply `None` entries via `.with_ranked_series` or `.with_raw_series_opt` to produce line breaks at absent time points:

```rust,no_run
let plot = BumpPlot::new()
    .with_ranked_series("Alpha", vec![Some(1.0), None, Some(2.0), Some(1.0)])
    .with_x_labels(["A", "B", "C", "D"]);
```

## Tie-breaking modes

| Mode | Behavior |
|------|----------|
| `Average` (default) | Tied series share the average of the occupied rank positions (e.g. 2.5, 2.5). |
| `Min` | All tied series receive the best (minimum) rank number. |
| `Max` | All tied series receive the worst (maximum) rank number. |
| `Stable` | Tied series retain their insertion order. |

**See also:** [Slope Chart](./slope.md) for a two-point-in-time version, [Parallel Coordinates](./parallel.md) for ranks across more than a time axis.

---

## CLI

Render a bump chart from a tabular file.

### Input format

Three columns: series name, time/condition label, rank (or raw value with `--raw-value`).

```
series  time   rank
Alpha   2021   1
Alpha   2022   3
Beta    2021   2
Beta    2022   1
Gamma   2021   3
Gamma   2022   2
```

### Usage

```
kuva bump [OPTIONS] [INPUT]
```

### Data columns

| Flag | Default | Description |
|------|---------|-------------|
| `--series <COL>` | `0` | Series name column (name or 0-based index). |
| `--time <COL>` | `1` | Time / condition label column. |
| `--rank <COL>` | `2` | Rank column (pre-ranked data). |
| `--raw-value` | off | Treat the rank column as a raw value and auto-compute ranks per time point. |
| `--rank-ascending` | off | With `--raw-value`: lower value → better (lower) rank number. |
| `--tie-break <MODE>` | `average` | Tie-breaking for auto-ranking: `average`, `min`, `max`, `stable`. |

### Appearance

| Flag | Default | Description |
|------|---------|-------------|
| `--curve <STYLE>` | `sigmoid` | Line style: `sigmoid` or `straight`. |
| `--rank-labels` | off | Draw the rank number inside each dot. |
| `--no-series-labels` | off | Hide the series name labels at the left/right edges. |
| `--dot-radius <F>` | `6.0` | Dot radius in pixels. |
| `--stroke-width <F>` | `2.5` | Line stroke width in pixels. |
| `--highlight <NAME>` | — | Highlight one series by name; all others are muted. |
| `--no-legend` | off | Hide the legend. |

### Examples

```bash
# Basic pre-ranked data
kuva bump data.tsv --series series --time year --rank rank -o bump.svg

# Auto-rank from scores (higher = better)
kuva bump scores.tsv --series team --time season --rank score --raw-value -o bump.svg

# Lower score is better (e.g. race times)
kuva bump times.tsv --series athlete --time race --rank time \
    --raw-value --rank-ascending -o bump.svg

# Highlight one series
kuva bump data.tsv --highlight "Alpha" -o bump.svg

# Sigmoid curves with rank labels inside dots
kuva bump data.tsv --curve sigmoid --rank-labels -o bump.svg
```

---

*See also: [Shared flags](../cli/index.md#shared-flags) — output, appearance, axes.*
