/// CLI twin-Y (dual-axis) subcommand (issue #106): `kuva twin-y` renders two
/// series sharing an x-axis but independent primary/secondary y-axes.
use std::io::Write;
use std::process::{Command, Stdio};

fn kuva_bin() -> Command {
    let bin = env!("CARGO_BIN_EXE_kuva");
    Command::new(bin)
}

fn run_with_stdin(args: &[&str], input: &str) -> (String, String, i32) {
    let mut cmd = kuva_bin();
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("failed to spawn kuva");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("write stdin");

    let out = child.wait_with_output().expect("wait");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

const WEATHER_TSV: &str = "month\ttemp\train\n\
1\t5\t80\n\
2\t8\t60\n\
3\t14\t45\n\
4\t20\t30\n\
5\t24\t20\n\
6\t22\t35\n";

#[test]
fn basic_twin_y_renders_both_axes() {
    let (stdout, stderr, code) = run_with_stdin(
        &[
            "twin-y",
            "--x",
            "month",
            "--y",
            "temp",
            "--y2",
            "rain",
            "--y-label",
            "Temp (C)",
            "--y2-label",
            "Rain (mm)",
        ],
        WEATHER_TSV,
    );
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.contains("Temp (C)"),
        "primary label missing: {stdout}"
    );
    assert!(
        stdout.contains("Rain (mm)"),
        "secondary label missing: {stdout}"
    );
    // Two independent series → two <path> line renders.
    assert_eq!(
        stdout.matches("<path").count(),
        2,
        "expected one path per series: {stdout}"
    );
}

#[test]
fn defaults_use_columns_0_1_2() {
    // No --x/--y/--y2: falls back to index 0/1/2, matching every other subcommand.
    let (stdout, stderr, code) = run_with_stdin(&["twin-y"], WEATHER_TSV);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("<svg"));
}

#[test]
fn legend_uses_column_names_by_default() {
    let (stdout, stderr, code) = run_with_stdin(
        &[
            "twin-y", "--x", "month", "--y", "temp", "--y2", "rain", "--legend",
        ],
        WEATHER_TSV,
    );
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.contains(">temp<"),
        "legend should default to column name: {stdout}"
    );
    assert!(
        stdout.contains(">rain<"),
        "legend should default to column name: {stdout}"
    );
}

#[test]
fn explicit_legend_labels_override_column_names() {
    let (stdout, stderr, code) = run_with_stdin(
        &[
            "twin-y",
            "--x",
            "month",
            "--y",
            "temp",
            "--y2",
            "rain",
            "--legend",
            "--primary-legend",
            "Temperature",
            "--secondary-legend",
            "Rainfall",
        ],
        WEATHER_TSV,
    );
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains(">Temperature<"));
    assert!(stdout.contains(">Rainfall<"));
    assert!(!stdout.contains(">temp<"));
}

#[test]
fn scatter_type_on_either_axis() {
    let (stdout, stderr, code) = run_with_stdin(
        &[
            "twin-y",
            "--x",
            "month",
            "--y",
            "temp",
            "--y2",
            "rain",
            "--primary-type",
            "scatter",
            "--secondary-type",
            "scatter",
        ],
        WEATHER_TSV,
    );
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.contains("<circle"),
        "scatter series should render circles: {stdout}"
    );
}

#[test]
fn unknown_plot_type_reports_a_clear_error() {
    let (_stdout, stderr, code) = run_with_stdin(
        &[
            "twin-y",
            "--x",
            "month",
            "--y",
            "temp",
            "--y2",
            "rain",
            "--primary-type",
            "bar",
        ],
        WEATHER_TSV,
    );
    assert_ne!(
        code, 0,
        "bar is not yet supported as a twin-y CLI plot type"
    );
    assert!(
        stderr.contains("line") && stderr.contains("scatter"),
        "error should name the supported types: {stderr}"
    );
}

#[test]
fn y2_axis_min_max_is_an_unconditional_override() {
    // Regression guard for the with_y2_range capping bug found while building
    // this feature: --y2-min/--y2-max must land exactly on the requested
    // bounds, not get pulled back near the secondary series' own data range.
    let (stdout, stderr, code) = run_with_stdin(
        &[
            "twin-y", "--x", "month", "--y", "temp", "--y2", "rain", "--y2-min", "0", "--y2-max",
            "200",
        ],
        WEATHER_TSV,
    );
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.contains(">200<"),
        "expected the secondary axis to reach the explicit 200 upper bound: {stdout}"
    );
}

#[test]
fn log_y2_applies_only_to_the_secondary_axis() {
    let (stdout, stderr, code) = run_with_stdin(
        &[
            "twin-y", "--x", "month", "--y", "temp", "--y2", "rain", "--log-y2",
        ],
        WEATHER_TSV,
    );
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("<svg"));
}
