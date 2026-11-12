use clap::Args;

use kuva::plot::pareto::ParetoPlot;
use kuva::render::layout::Layout;
use kuva::render::plots::Plot;
use kuva::render::render::render_multiple;

use crate::data::{ColSpec, DataTable, InputArgs};
use crate::layout_args::{apply_axis_args, apply_base_args, AxisArgs, BaseArgs};
use crate::output::write_output;

/// Pareto chart — bar chart sorted descending by value, with a cumulative-percentage
/// line on a secondary axis (the "80/20 rule" chart).
#[derive(Args, Debug)]
pub struct ParetoArgs {
    /// Label column (0-based index or header name; default: 0).
    #[arg(long)]
    pub label_col: Option<ColSpec>,

    /// Value column (0-based index or header name; default: 1).
    #[arg(long)]
    pub value_col: Option<ColSpec>,

    /// Bar fill color (CSS string; default: "steelblue").
    #[arg(long)]
    pub color: Option<String>,

    /// Cumulative-line color (CSS string; default: "firebrick").
    #[arg(long)]
    pub line_color: Option<String>,

    /// Bar width as a fraction of the slot (default: 0.8).
    #[arg(long)]
    pub bar_width: Option<f64>,

    /// Preserve input row order instead of sorting descending by value.
    #[arg(long)]
    pub no_sort: bool,

    /// Threshold reference line, as a cumulative percentage (default: 80.0).
    #[arg(long)]
    pub threshold: Option<f64>,

    /// Hide the threshold reference line.
    #[arg(long)]
    pub no_threshold: bool,

    /// Show a percentage label above each cumulative-line point.
    #[arg(long)]
    pub cumulative_labels: bool,

    /// Legend labels for the bars and the cumulative line: "BAR_LABEL,LINE_LABEL"
    /// (shown by default as "Value,Cumulative %"; this overrides the text).
    #[arg(long, value_delimiter = ',')]
    pub legend: Vec<String>,

    /// Hide the legend (shown by default).
    #[arg(long)]
    pub no_legend: bool,

    /// Collapse categories beyond this count into one stacked "Other" bar
    /// (counts the bucket itself: --max-categories 5 keeps the top 4 plus one
    /// "Other" bar for the rest). Unset by default (no bucketing).
    #[arg(long)]
    pub max_categories: Option<usize>,

    /// Label for the bucketed bar (default: "Other"). No effect without
    /// --max-categories.
    #[arg(long)]
    pub other_label: Option<String>,

    /// Render categories on the Y-axis and values on the X-axis. The cumulative-%
    /// line moves to a secondary X-axis drawn on top of the plot.
    #[arg(long)]
    pub horizontal: bool,

    #[command(flatten)]
    pub input: InputArgs,

    #[command(flatten)]
    pub base: BaseArgs,

    #[command(flatten)]
    pub axis: AxisArgs,
}

pub fn run(args: ParetoArgs) -> Result<(), String> {
    let label_col = args.label_col.clone().unwrap_or(ColSpec::Index(0));
    let value_col = args.value_col.clone().unwrap_or(ColSpec::Index(1));
    let proj: Vec<ColSpec> = vec![label_col.clone(), value_col.clone()];
    let table = DataTable::parse(
        args.input.input.as_deref(),
        args.input.header_mode(),
        args.input.delimiter,
        &proj,
    )?;

    let labels = table.col_str(&label_col)?;
    let values = table.col_f64(&value_col)?;

    let mut plot = ParetoPlot::new().with_categories(labels.into_iter().zip(values));

    if let Some(s) = args.color {
        plot = plot.with_color(s);
    }
    if let Some(s) = args.line_color {
        plot = plot.with_line_color(s);
    }
    if let Some(w) = args.bar_width {
        plot = plot.with_width(w);
    }
    if args.no_sort {
        plot = plot.with_sorted(false);
    }
    if let Some(t) = args.threshold {
        plot = plot.with_threshold(t);
    }
    if args.no_threshold {
        plot = plot.with_show_threshold(false);
    }
    if args.cumulative_labels {
        plot = plot.with_cumulative_labels(true);
    }
    if let [bar_label, line_label] = args.legend.as_slice() {
        plot = plot.with_legend(bar_label.clone(), line_label.clone());
    }
    if args.no_legend {
        plot = plot.with_show_legend(false);
    }
    if let Some(other_label) = args.other_label {
        plot = plot.with_other_label(other_label);
    }
    if let Some(max) = args.max_categories {
        plot = plot.with_max_categories(max);
    }
    if args.horizontal {
        plot = plot.with_horizontal(true);
    }

    #[cfg(feature = "emit_code")]
    if args.base.emit_code {
        print!(
            "{}",
            crate::emit_code::assemble(
                &["kuva::plot::ParetoPlot"],
                "Pareto",
                &[crate::emit_code::emit_pareto_plot(&plot)],
                &args.base,
                Some(&args.axis),
                None,
            )
        );
        return Ok(());
    }

    let plots = vec![Plot::Pareto(plot)];
    // -45° rotation is now `auto_from_plots`'s own default for Pareto (so library
    // callers get it too, not just the CLI) -- `apply_axis_args` below still lets
    // an explicit `--x-tick-rotate` flag override it, which a hardcoded call here
    // would have clobbered.
    let layout = Layout::auto_from_plots(&plots);
    let layout = apply_base_args(layout, &args.base);
    let layout = apply_axis_args(layout, &args.axis);
    let scene = render_multiple(plots, layout);
    write_output(scene, &args.base)
}
