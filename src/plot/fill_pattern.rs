/// Pattern fill for filled areas, bars, and other solid regions.
///
/// The default is [`FillPattern::Solid`] (no pattern, plain color fill).
///
/// Patterns are fully independent of the fill color — the hatch lines are
/// always black and render as an overlay on top of whatever color fill is
/// applied. This means every combination of color and pattern is valid, and
/// patterns remain useful even after converting a figure to greyscale.
///
/// When used with [`crate::render::Layout::with_bw_mode()`], patterns and grey
/// shades are assigned automatically to maximise distinguishability without
/// relying on color.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FillPattern {
    /// No pattern — plain solid fill (default).
    #[default]
    Solid,
    /// Horizontal parallel lines (═══).
    Horizontal,
    /// Vertical parallel lines (|||).
    Vertical,
    /// Forward-diagonal lines (///).
    DiagonalForward,
    /// Back-diagonal lines (\\\).
    DiagonalBack,
    /// Horizontal + vertical grid (+++).
    Crosshatch,
    /// Forward + back diagonal grid (×××).
    DiagonalCrosshatch,
    /// Scattered filled dots (···).
    Dots,
}

impl FillPattern {
    /// The SVG `id` used to reference this pattern via `fill="url(#id)"`.
    ///
    /// Returns an empty string for [`FillPattern::Solid`].
    pub fn id(self) -> &'static str {
        match self {
            FillPattern::Solid => "",
            FillPattern::Horizontal => "kuva-fp-horiz",
            FillPattern::Vertical => "kuva-fp-vert",
            FillPattern::DiagonalForward => "kuva-fp-diag-fwd",
            FillPattern::DiagonalBack => "kuva-fp-diag-back",
            FillPattern::Crosshatch => "kuva-fp-crosshatch",
            FillPattern::DiagonalCrosshatch => "kuva-fp-diag-cross",
            FillPattern::Dots => "kuva-fp-dots",
        }
    }

    /// The complete SVG `<pattern>…</pattern>` element to place in `<defs>`.
    ///
    /// The pattern has a transparent background so the base fill color shows
    /// through. Hatch lines are black and sized for readability at typical plot
    /// resolutions and at 300 DPI print output.
    ///
    /// Returns an empty string for [`FillPattern::Solid`].
    pub fn svg_def(self) -> &'static str {
        match self {
            FillPattern::Solid => "",

            FillPattern::Horizontal => concat!(
                r#"<pattern id="kuva-fp-horiz" patternUnits="userSpaceOnUse" width="8" height="6">"#,
                r#"<line x1="0" y1="3" x2="8" y2="3" stroke="black" stroke-width="0.6"/>"#,
                r#"</pattern>"#,
            ),

            FillPattern::Vertical => concat!(
                r#"<pattern id="kuva-fp-vert" patternUnits="userSpaceOnUse" width="6" height="8">"#,
                r#"<line x1="3" y1="0" x2="3" y2="8" stroke="black" stroke-width="0.6"/>"#,
                r#"</pattern>"#,
            ),

            // Three path segments tile a continuous diagonal line across any
            // shape without gaps: top-left corner, main stripe, bottom-right corner.
            FillPattern::DiagonalForward => concat!(
                r#"<pattern id="kuva-fp-diag-fwd" patternUnits="userSpaceOnUse" width="6" height="6">"#,
                r#"<path d="M-1,1 l2,-2 M0,6 l6,-6 M5,7 l2,-2" stroke="black" stroke-width="0.6" fill="none"/>"#,
                r#"</pattern>"#,
            ),

            FillPattern::DiagonalBack => concat!(
                r#"<pattern id="kuva-fp-diag-back" patternUnits="userSpaceOnUse" width="6" height="6">"#,
                r#"<path d="M-1,5 l2,2 M0,0 l6,6 M5,-1 l2,2" stroke="black" stroke-width="0.6" fill="none"/>"#,
                r#"</pattern>"#,
            ),

            FillPattern::Crosshatch => concat!(
                r#"<pattern id="kuva-fp-crosshatch" patternUnits="userSpaceOnUse" width="8" height="8">"#,
                r#"<path d="M0,4 H8 M4,0 V8" stroke="black" stroke-width="0.6" fill="none"/>"#,
                r#"</pattern>"#,
            ),

            FillPattern::DiagonalCrosshatch => concat!(
                r#"<pattern id="kuva-fp-diag-cross" patternUnits="userSpaceOnUse" width="6" height="6">"#,
                r#"<path d="M-1,1 l2,-2 M0,6 l6,-6 M5,7 l2,-2 M-1,5 l2,2 M0,0 l6,6 M5,-1 l2,2" stroke="black" stroke-width="0.6" fill="none"/>"#,
                r#"</pattern>"#,
            ),

            FillPattern::Dots => concat!(
                r#"<pattern id="kuva-fp-dots" patternUnits="userSpaceOnUse" width="8" height="8">"#,
                r#"<circle cx="4" cy="4" r="0.9" fill="black"/>"#,
                r#"</pattern>"#,
            ),
        }
    }

    /// Returns `true` for any variant other than [`FillPattern::Solid`].
    pub fn is_patterned(self) -> bool {
        self != FillPattern::Solid
    }

    /// Reverse of [`FillPattern::id`] — looks up the variant from its SVG
    /// pattern `id`. Used by backends that can't resolve an SVG `<pattern>`
    /// paint server (e.g. the raster backend) and need to identify which
    /// hatch a `fill="url(#id)"` reference names.
    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "kuva-fp-horiz" => FillPattern::Horizontal,
            "kuva-fp-vert" => FillPattern::Vertical,
            "kuva-fp-diag-fwd" => FillPattern::DiagonalForward,
            "kuva-fp-diag-back" => FillPattern::DiagonalBack,
            "kuva-fp-crosshatch" => FillPattern::Crosshatch,
            "kuva-fp-diag-cross" => FillPattern::DiagonalCrosshatch,
            "kuva-fp-dots" => FillPattern::Dots,
            _ => return None,
        })
    }

    /// Ink coverage (`0.0` transparent .. `1.0` solid black) of this pattern's
    /// hatch at local point `(x, y)`, in the same coordinate units as
    /// [`FillPattern::svg_def`]'s tile (unscaled user-space units, tiled from
    /// the origin). For backends that raster shapes directly instead of
    /// resolving an SVG `<pattern>` paint server.
    pub fn hatch_coverage(self, x: f32, y: f32) -> f32 {
        // Matches `stroke-width="0.6"` in `svg_def`.
        const HALF_STROKE: f32 = 0.3;
        // Antialiasing ramp width, in the same user-space units as the tile.
        const AA: f32 = 0.5;

        // Distance from `v` to the nearest point where `(v - center)` is a
        // multiple of `period` (i.e. distance to the nearest repeat of a
        // 1-D line family), folded into `[0, period/2]`.
        fn line_dist_1d(v: f32, period: f32, center: f32) -> f32 {
            let m = (v - center).rem_euclid(period);
            m.min(period - m)
        }

        // Coverage from a perpendicular distance to a stroked line: full ink
        // within HALF_STROKE, ramping to zero over the next AA units.
        fn stroke_ramp(dist: f32) -> f32 {
            ((HALF_STROKE + AA - dist) / AA).clamp(0.0, 1.0)
        }

        // Coverage from a distance to a filled disk of radius `r`.
        fn disk_ramp(dist: f32, r: f32) -> f32 {
            ((r + AA - dist) / AA).clamp(0.0, 1.0)
        }

        match self {
            FillPattern::Solid => 0.0,

            // Line spans the full tile width; only y repeats.
            FillPattern::Horizontal => stroke_ramp(line_dist_1d(y, 6.0, 3.0)),
            // Line spans the full tile height; only x repeats.
            FillPattern::Vertical => stroke_ramp(line_dist_1d(x, 6.0, 3.0)),

            // Diagonal line family x + y ≡ 0 (mod 6); perpendicular distance
            // divides the (x+y)-distance by sqrt(2).
            FillPattern::DiagonalForward => {
                stroke_ramp(line_dist_1d(x + y, 6.0, 0.0) / std::f32::consts::SQRT_2)
            }
            // Diagonal line family x - y ≡ 0 (mod 6).
            FillPattern::DiagonalBack => {
                stroke_ramp(line_dist_1d(x - y, 6.0, 0.0) / std::f32::consts::SQRT_2)
            }

            // Horizontal line (period 8, y=4) union vertical line (period 8, x=4).
            FillPattern::Crosshatch => {
                let h = stroke_ramp(line_dist_1d(y, 8.0, 4.0));
                let v = stroke_ramp(line_dist_1d(x, 8.0, 4.0));
                h.max(v)
            }

            // Union of both diagonal families, period 6.
            FillPattern::DiagonalCrosshatch => {
                let fwd = stroke_ramp(line_dist_1d(x + y, 6.0, 0.0) / std::f32::consts::SQRT_2);
                let back = stroke_ramp(line_dist_1d(x - y, 6.0, 0.0) / std::f32::consts::SQRT_2);
                fwd.max(back)
            }

            // Filled disk of radius 0.9 centered in an 8x8 tile.
            FillPattern::Dots => {
                let xm = x.rem_euclid(8.0) - 4.0;
                let ym = y.rem_euclid(8.0) - 4.0;
                disk_ramp((xm * xm + ym * ym).sqrt(), 0.9)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_id_round_trips_with_id() {
        for p in [
            FillPattern::Horizontal,
            FillPattern::Vertical,
            FillPattern::DiagonalForward,
            FillPattern::DiagonalBack,
            FillPattern::Crosshatch,
            FillPattern::DiagonalCrosshatch,
            FillPattern::Dots,
        ] {
            assert_eq!(
                FillPattern::from_id(p.id()),
                Some(p),
                "{p:?} should round-trip through its id"
            );
        }
    }

    #[test]
    fn from_id_rejects_unknown_and_solid() {
        assert_eq!(FillPattern::from_id("not-a-pattern"), None);
        // Solid has an empty id and is never referenced via url(#...), so it
        // must not be reachable through from_id.
        assert_eq!(FillPattern::from_id(""), None);
    }

    #[test]
    fn solid_has_zero_coverage_everywhere() {
        for (x, y) in [(0.0, 0.0), (3.0, 3.0), (100.0, -50.0)] {
            assert_eq!(FillPattern::Solid.hatch_coverage(x, y), 0.0);
        }
    }

    #[test]
    fn horizontal_hatch_peaks_on_line_and_fades_between() {
        // svg_def: line at y=3, tile height 6 — on-line at y=3, off-line
        // (farthest from any line) at y=6 (equidistant between y=3 and y=9).
        assert_eq!(FillPattern::Horizontal.hatch_coverage(0.0, 3.0), 1.0);
        assert_eq!(FillPattern::Horizontal.hatch_coverage(0.0, 6.0), 0.0);
        // Periodic: y=3 and y=9 (one tile over) are both on-line.
        assert_eq!(FillPattern::Horizontal.hatch_coverage(0.0, 9.0), 1.0);
        // x doesn't matter for a full-width horizontal line.
        assert_eq!(
            FillPattern::Horizontal.hatch_coverage(0.0, 3.0),
            FillPattern::Horizontal.hatch_coverage(1000.0, 3.0),
        );
    }

    #[test]
    fn vertical_hatch_peaks_on_line_and_fades_between() {
        assert_eq!(FillPattern::Vertical.hatch_coverage(3.0, 0.0), 1.0);
        assert_eq!(FillPattern::Vertical.hatch_coverage(6.0, 0.0), 0.0);
    }

    #[test]
    fn diagonal_forward_and_back_are_distinct() {
        // At (0,0): forward's line family is x+y=0 (on-line, full coverage);
        // back's line family is x-y=0 (also on-line here) — pick a point
        // where they diverge: (1, 0). Forward: x+y=1, dist to nearest
        // multiple of 6 is 1 (not on-line). Back: x-y=1, same. Use (3, -3):
        // forward x+y=0 -> on-line; back x-y=6 -> on-line too. Use (1, 2):
        // forward x+y=3 -> off-line (dist 3, max off); back x-y=-1 -> dist 1.
        let fwd = FillPattern::DiagonalForward.hatch_coverage(1.0, 2.0);
        let back = FillPattern::DiagonalBack.hatch_coverage(1.0, 2.0);
        assert!(
            fwd < back,
            "forward ({fwd}) should be farther off-line than back ({back}) at (1,2)"
        );
    }

    #[test]
    fn crosshatch_covers_both_diagonal_and_horizontal_lines() {
        // Crosshatch tile is 8x8 with lines at x=4 and y=4.
        assert_eq!(FillPattern::Crosshatch.hatch_coverage(4.0, 0.0), 1.0);
        assert_eq!(FillPattern::Crosshatch.hatch_coverage(0.0, 4.0), 1.0);
        // Off both lines and far from either (corner of the tile).
        assert_eq!(FillPattern::Crosshatch.hatch_coverage(0.0, 0.0), 0.0);
    }

    #[test]
    fn dots_peak_at_tile_center_and_fade_at_tile_corner() {
        // Dots tile is 8x8 with a disk at (4,4), r=0.9.
        assert_eq!(FillPattern::Dots.hatch_coverage(4.0, 4.0), 1.0);
        assert_eq!(FillPattern::Dots.hatch_coverage(0.0, 0.0), 0.0);
        // Periodic: (12, 12) is the same tile-local point as (4, 4).
        assert_eq!(FillPattern::Dots.hatch_coverage(12.0, 12.0), 1.0);
    }
}
