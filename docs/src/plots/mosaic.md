# Mosaic Plot

A mosaic (Marimekko) chart encodes two categorical variables simultaneously. Column widths are proportional to column totals, and the height of each segment within a column represents that row category's share. Each cell's **area** is therefore proportional to its joint frequency — making it an area-encoded contingency table.

Mosaic plots are used in clinical research, survey analysis, and A/B testing to visualize the relationship between two categorical variables across different group sizes.

**Import path:** `kuva::plot::mosaic::MosaicPlot`

---

## Basic usage

Use `.with_cell(col, row, value)` to add one cell per combination. Column order follows first-seen order; use `.with_col_order()` to set it explicitly.

```rust,no_run
use kuva::plot::mosaic::MosaicPlot;
use kuva::backend::svg::SvgBackend;
use kuva::render::render::render_multiple;
use kuva::render::layout::Layout;
use kuva::render::plots::Plot;

let plot = MosaicPlot::new()
    .with_cell("Control",   "Positive", 28.0)
    .with_cell("Control",   "Negative", 72.0)
    .with_cell("Low dose",  "Positive", 45.0)
    .with_cell("Low dose",  "Negative", 55.0)
    .with_cell("High dose", "Positive", 68.0)
    .with_cell("High dose", "Negative", 32.0)
    .with_legend("Response");

let plots = vec![Plot::Mosaic(plot)];
let layout = Layout::auto_from_plots(&plots)
    .with_title("Treatment vs Response")
    .with_x_label("Dose")
    .with_y_label("Proportion");

let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
std::fs::write("mosaic.svg", svg).unwrap();
```

<img src="../assets/mosaic/basic.svg" alt="Basic mosaic plot showing treatment vs response" width="560">

Column widths reflect the total sample size in each dose group (wider = more subjects). Segment heights show the response breakdown within each column.

---

## Custom color and ordering

Assign explicit segment colors with `.with_group_colors()`, and control the display order of columns and rows with `.with_col_order()` and `.with_row_order()`.

```rust,no_run
use kuva::plot::mosaic::MosaicPlot;
use kuva::render::plots::Plot;
use kuva::render::layout::Layout;
use kuva::render::render::render_multiple;
use kuva::backend::svg::SvgBackend;

let plot = MosaicPlot::new()
    .with_cells([
        ("Q1", "Product A", 120.0), ("Q1", "Product B", 85.0), ("Q1", "Product C", 45.0),
        ("Q2", "Product A", 140.0), ("Q2", "Product B", 70.0), ("Q2", "Product C", 60.0),
        ("Q3", "Product A", 110.0), ("Q3", "Product B", 95.0), ("Q3", "Product C", 80.0),
        ("Q4", "Product A", 160.0), ("Q4", "Product B", 80.0), ("Q4", "Product C", 90.0),
    ])
    .with_col_order(["Q1", "Q2", "Q3", "Q4"])
    .with_row_order(["Product A", "Product B", "Product C"])
    .with_group_colors(["#1f77b4", "#ff7f0e", "#2ca02c"])
    .with_legend("Product");

let plots = vec![Plot::Mosaic(plot)];
let layout = Layout::auto_from_plots(&plots)
    .with_title("Sales Mix by Quarter")
    .with_x_label("Quarter")
    .with_y_label("Share");

let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
```

<img src="../assets/mosaic/quarterly.svg" alt="Mosaic plot showing quarterly product sales mix" width="560">

---

## Showing raw values

By default, cells show percentage labels. Toggle to raw values with `.with_values(true)` and suppress percentages with `.with_percents(false)`.

```rust,no_run
use kuva::plot::mosaic::MosaicPlot;
use kuva::render::plots::Plot;
# use kuva::render::layout::Layout;
# use kuva::render::render::render_multiple;

let plot = MosaicPlot::new()
    .with_cell("Smoker",     "Disease",    48.0)
    .with_cell("Smoker",     "No disease", 152.0)
    .with_cell("Non-smoker", "Disease",    22.0)
    .with_cell("Non-smoker", "No disease", 278.0)
    .with_percents(false)
    .with_values(true)
    .with_legend("Outcome");

let plots = vec![Plot::Mosaic(plot)];
```

---

## Non-normalized columns

`.with_normalize(false)` makes column heights proportional to their share of the grand total rather than filling the full plot height. This reveals differences in total group sizes while preserving the area proportionality.

```rust,no_run
use kuva::plot::mosaic::MosaicPlot;
use kuva::render::plots::Plot;
# use kuva::render::layout::Layout;
# use kuva::render::render::render_multiple;

// Very unequal group sizes: column heights will differ
let plot = MosaicPlot::new()
    .with_cell("Large group",  "Yes", 480.0)
    .with_cell("Large group",  "No",  320.0)
    .with_cell("Small group",  "Yes",  35.0)
    .with_cell("Small group",  "No",   65.0)
    .with_normalize(false)
    .with_legend("Response");

let plots = vec![Plot::Mosaic(plot)];
```

---

## MosaicPlot API reference

### `MosaicPlot` builders

| Method | Default | Description |
|--------|---------|-------------|
| `MosaicPlot::new()` | — | Create a mosaic plot with default settings |
| `.with_cell(col, row, value)` | — | Add a single cell |
| `.with_cells(iter)` | — | Add multiple `(col, row, value)` cells at once |
| `.with_col_order(iter)` | first-seen | Explicit column display order |
| `.with_row_order(iter)` | first-seen | Explicit row/segment display order |
| `.with_group_colors(iter)` | palette | Per-row CSS colors (indexed by row order) |
| `.with_gap(px)` | `2.0` | Pixel gap between columns and between segments |
| `.with_percents(bool)` | `true` | Show percentage labels inside cells |
| `.with_values(bool)` | `false` | Show raw value labels inside cells |
| `.with_normalize(bool)` | `true` | Normalize each column to fill full plot height |
| `.with_legend(label)` | — | Legend title (one entry per row category) |

**See also:** [Venn Diagram](./venn.md) and [UpSet Plot](./upset.md) for other set/category-overlap views, [Dice Plot](./diceplot.md) for a per-cell multivariate grid.

---

## CLI

Mosaic / Marimekko chart — two-way contingency table where column widths encode one marginal and stacked segments encode the other.

**Input:** one row per cell with column-category, row-category, and count/value columns.

| Flag | Default | Description |
|---|---|---|
| `--col-col <COL>` | `0` | Column-category column (determines bar widths) |
| `--row-col <COL>` | `1` | Row-category column (determines stacked segments) |
| `--value-col <COL>` | `2` | Count or value column |
| `--gap <PX>` | `2.0` | Pixel gap between tiles |
| `--no-percents` | off | Hide percentage labels inside tiles |
| `--show-values` | off | Show raw values inside tiles |
| `--legend <LABEL>` | — | Add a legend |

```bash
kuva mosaic data.tsv --col-col region --row-col outcome --value-col count

kuva mosaic data.tsv --col-col region --row-col outcome --value-col count \
    --show-values --title "Outcomes by Region"
```

---

*See also: [Shared flags](../cli/index.md#shared-flags) — output, appearance, axes.*
