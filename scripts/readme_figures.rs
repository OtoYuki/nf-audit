//! The README's figures, drawn from the megatest corpus with nf-audit's own parsers.
//!
//! `cargo run --release --example readme-figures -- <out-dir>` writes a light and a dark SVG of
//! each figure. `scripts/regen-examples.sh` runs it, so `--check` also catches a figure that no
//! longer matches the data. Needs the corpus: `scripts/pull-megatests.sh --corpus`.

use nf_audit::analysis::Rates;
use nf_audit::compare::load_runs;
use nf_audit::input::read_report_with_meta;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Colours by role, from the nf-audit brand (ink #141C10, cream #FBFFE1, khaki #D1CF8B,
/// olive #99920B, moss #5A6042). As in the logo, the filled segment is what was used (moss on
/// cream, khaki on ink) and olive is what was reserved and never used.
struct Theme {
    name: &'static str,
    surface: &'static str,
    text: &'static str,
    muted: &'static str,
    grid: &'static str,
    series1: &'static str,
    series2: &'static str,
}

const LIGHT: Theme = Theme {
    name: "light",
    surface: "#FBFFE1",
    text: "#141C10",
    muted: "#5A6042",
    grid: "#E3E5C3",
    series1: "#5A6042",
    series2: "#99920B",
};
const DARK: Theme = Theme {
    name: "dark",
    surface: "#141C10",
    text: "#FBFFE1",
    muted: "#D1CF8B",
    grid: "#2A3322",
    series1: "#D1CF8B",
    series2: "#99920B",
};

const FONT: &str = "'Geist Mono', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn list(name: &str) -> Vec<PathBuf> {
    std::fs::read_to_string(root().join("examples").join(name))
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| root().join(l))
        .collect()
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn open(w: f64, h: f64, t: &Theme, title: &str, desc: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" role=\"img\" aria-labelledby=\"t d\" font-family=\"{FONT}\">\n\
         <title id=\"t\">{}</title>\n<desc id=\"d\">{}</desc>\n\
         <rect width=\"{w}\" height=\"{h}\" rx=\"8\" fill=\"{}\"/>\n",
        esc(title),
        esc(desc),
        t.surface
    )
}

// Position, size, colour, anchor and weight are all per-call; a struct would only rename them.
#[allow(clippy::too_many_arguments)]
fn text(
    s: &mut String,
    x: f64,
    y: f64,
    size: f64,
    fill: &str,
    anchor: &str,
    weight: u32,
    body: &str,
) {
    let _ = writeln!(
        s,
        "<text x=\"{x:.1}\" y=\"{y:.1}\" font-size=\"{size}\" fill=\"{fill}\" text-anchor=\"{anchor}\" font-weight=\"{weight}\">{}</text>",
        esc(body)
    );
}

/// A column segment: square at the bottom, 4 px rounded top when `round_top`.
fn column(s: &mut String, x: f64, y: f64, w: f64, h: f64, fill: &str, round_top: bool) {
    if h <= 0.0 {
        return;
    }
    let r = if round_top {
        4f64.min(h).min(w / 2.0)
    } else {
        0.0
    };
    let _ = writeln!(
        s,
        "<path d=\"M{x:.2},{b:.2}V{t1:.2}Q{x:.2},{y:.2} {x1:.2},{y:.2}H{x2:.2}Q{xr:.2},{y:.2} {xr:.2},{t1:.2}V{b:.2}Z\" fill=\"{fill}\"/>",
        b = y + h,
        t1 = y + r,
        x1 = x + r,
        x2 = x + w - r,
        xr = x + w,
    );
}

/// Figure 1: cost of each `star_salmon` release's full-size test run, split into what was
/// used and what was reserved but never used.
fn salmon_figure(t: &Theme) -> String {
    let runs = load_runs(
        &list("rnaseq-star_salmon-releases.files"),
        &Rates::SEQERA_COMPUTE,
    )
    .unwrap();
    let (w, h) = (880.0, 418.0);
    let (left, right, top, bottom) = (58.0, 20.0, 122.0, 44.0);
    let (pw, ph) = (w - left - right, h - top - bottom);
    let ymax = 350.0;
    let y = |v: f64| top + ph - v / ymax * ph;
    let slot = pw / runs.len() as f64;
    let bw = (slot - 6.0).min(24.0);

    let in_band = runs
        .iter()
        .filter(|r| {
            let m = r.stats.cpu_h_req_metered * 0.10 + r.stats.gib_h_req_metered * 0.025;
            (62.0..=68.0).contains(&(r.stats.total_waste / m * 100.0).round())
        })
        .count();
    let title = "nf-core/rnaseq full-size test, star_salmon: cost per release";
    let desc = format!(
        "Stacked columns for {} releases from {} to {}. Cost fell from ${:.0} to ${:.0}; the part reserved but never used stayed at 62 to 68 percent of metered cost in {} of them.",
        runs.len(),
        runs[0].meta.revision.as_deref().unwrap_or("?"),
        runs[runs.len() - 1].meta.revision.as_deref().unwrap_or("?"),
        runs[0].stats.total_cost,
        runs[runs.len() - 1].stats.total_cost,
        in_band
    );
    let mut s = open(w, h, t, title, &desc);
    text(&mut s, left, 30.0, 16.0, t.text, "start", 600, title);
    text(
        &mut s,
        left,
        52.0,
        13.0,
        t.muted,
        "start",
        400,
        "One run per release, priced at Seqera Compute list rates.",
    );
    text(
        &mut s,
        left,
        70.0,
        13.0,
        t.muted,
        "start",
        400,
        &format!(
            "The idle share stayed at 62–68% in {in_band} of {} releases while the cost fell.",
            runs.len()
        ),
    );
    // Legend
    let lx = left;
    for (i, (label, fill)) in [("used", t.series1), ("reserved, never used", t.series2)]
        .iter()
        .enumerate()
    {
        let x = lx + i as f64 * 110.0;
        let _ = writeln!(
            s,
            "<rect x=\"{x}\" y=\"88\" width=\"12\" height=\"12\" rx=\"3\" fill=\"{fill}\"/>"
        );
        text(&mut s, x + 18.0, 98.5, 12.5, t.text, "start", 400, label);
    }
    // Grid and y ticks
    for v in [0.0, 100.0, 200.0, 300.0] {
        let yy = y(v);
        let _ = writeln!(
            s,
            "<line x1=\"{left}\" x2=\"{:.1}\" y1=\"{yy:.1}\" y2=\"{yy:.1}\" stroke=\"{}\" stroke-width=\"1\"/>",
            w - right,
            t.grid
        );
        text(
            &mut s,
            left - 8.0,
            yy + 4.0,
            12.0,
            t.muted,
            "end",
            400,
            &format!("${v:.0}"),
        );
    }
    let show = ["3.1", "3.5", "3.9", "3.11.1", "3.15.1", "3.22.0", "3.26.0"];
    for (i, r) in runs.iter().enumerate() {
        let x = left + i as f64 * slot + (slot - bw) / 2.0;
        let total = r.stats.total_cost;
        let unused = r.stats.total_waste;
        let used = total - unused;
        let yu = y(used);
        column(&mut s, x, yu, bw, top + ph - yu, t.series1, false);
        // 2 px surface gap between the segments.
        let yt = y(total);
        column(&mut s, x, yt, bw, (yu - 2.0) - yt, t.series2, true);
        let rev = r.meta.revision.as_deref().unwrap_or("");
        if show.contains(&rev) {
            text(
                &mut s,
                x + bw / 2.0,
                h - bottom + 18.0,
                12.0,
                t.muted,
                "middle",
                400,
                rev,
            );
        }
        // Direct labels: the first run, the last, and the -resume outlier.
        if i == 0 || i == runs.len() - 1 {
            text(
                &mut s,
                x + bw / 2.0,
                yt - 7.0,
                12.0,
                t.text,
                "middle",
                600,
                &format!("${total:.0}"),
            );
        }
        if rev == "3.5" {
            text(
                &mut s,
                x + bw + 6.0,
                yt + 10.0,
                12.0,
                t.muted,
                "start",
                400,
                "3.5: a -resume run",
            );
        }
    }
    s.push_str("</svg>\n");
    s
}

/// Figure 2: average cores used by each completed `RSEM_CALCULATEEXPRESSION` task from 3.22.0
/// on, against the 12 CPUs each one reserved.
fn rsem_figure(t: &Theme) -> String {
    let mut groups: Vec<(String, Vec<f64>)> = Vec::new();
    for p in list("rnaseq-star_rsem-releases.files") {
        let (tasks, meta) = read_report_with_meta(&p).unwrap();
        let rev = meta.revision.unwrap_or_default();
        let v: Vec<u32> = rev.split('.').filter_map(|x| x.parse().ok()).collect();
        if v.len() != 3 || v < vec![3, 22, 0] {
            continue;
        }
        let mut cores: Vec<f64> = tasks
            .iter()
            .filter(|t| t.process.ends_with(":RSEM_CALCULATEEXPRESSION") && t.status == "COMPLETED")
            .filter_map(|t| t.pct_cpu.map(|p| p / 100.0))
            .collect();
        cores.sort_by(f64::total_cmp);
        groups.push((rev, cores));
    }
    let mut all: Vec<f64> = groups.iter().flat_map(|g| g.1.iter().copied()).collect();
    all.sort_by(f64::total_cmp);
    let n = all.len();
    let median = (all[(n - 1) / 2] + all[n / 2]) / 2.0;

    let (w, h) = (880.0, 360.0);
    let (left, right, top, bottom) = (58.0, 150.0, 84.0, 44.0);
    let (pw, ph) = (w - left - right, h - top - bottom);
    let ymax = 13.0;
    let y = |v: f64| top + ph - v / ymax * ph;
    let title = "RSEM_CALCULATEEXPRESSION: cores used vs. cores reserved";
    let desc = format!(
        "Average cores used by each of the {n} completed tasks in rnaseq {} to {}, one dot per task, against the 12 CPUs each task reserved. Median {median:.2} cores.",
        groups[0].0,
        groups[groups.len() - 1].0
    );
    let mut s = open(w, h, t, title, &desc);
    text(&mut s, left, 30.0, 16.0, t.text, "start", 600, title);
    text(
        &mut s,
        left,
        52.0,
        13.0,
        t.muted,
        "start",
        400,
        &format!("{n} completed tasks, rnaseq star_rsem full-size test, releases {}–{}. One dot per task.", groups[0].0, groups[groups.len() - 1].0),
    );
    for v in [0.0, 4.0, 8.0, 12.0] {
        let yy = y(v);
        let _ = writeln!(
            s,
            "<line x1=\"{left}\" x2=\"{:.1}\" y1=\"{yy:.1}\" y2=\"{yy:.1}\" stroke=\"{}\" stroke-width=\"1\"/>",
            left + pw,
            t.grid
        );
        text(
            &mut s,
            left - 8.0,
            yy + 4.0,
            12.0,
            t.muted,
            "end",
            400,
            &format!("{v:.0}"),
        );
    }
    // Reserved: 12 CPUs per task; median of what was used.
    for (v, label, weight) in [
        (12.0, "reserved: 12 CPUs".to_string(), 600),
        (median, format!("median used: {median:.2}"), 600),
    ] {
        let yy = y(v);
        let _ = writeln!(
            s,
            "<line x1=\"{left}\" x2=\"{:.1}\" y1=\"{yy:.1}\" y2=\"{yy:.1}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            left + pw,
            t.muted
        );
        text(
            &mut s,
            left + pw + 8.0,
            yy + 4.0,
            12.5,
            t.text,
            "start",
            weight,
            &label,
        );
    }
    let slot = pw / groups.len() as f64;
    for (gi, (rev, cores)) in groups.iter().enumerate() {
        let cx = left + (gi as f64 + 0.5) * slot;
        let k = cores.len() as f64;
        for (i, c) in cores.iter().enumerate() {
            // Spread each release's dots across its slot so they don't overlap.
            let dx = if k > 1.0 {
                (i as f64 / (k - 1.0) - 0.5) * slot * 0.5
            } else {
                0.0
            };
            let _ = writeln!(
                s,
                "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"4.5\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx + dx,
                y(*c),
                t.series1,
                t.surface
            );
        }
        text(
            &mut s,
            cx,
            h - bottom + 18.0,
            12.0,
            t.muted,
            "middle",
            400,
            rev,
        );
    }
    text(
        &mut s,
        left - 44.0,
        top - 10.0,
        12.0,
        t.muted,
        "start",
        400,
        "cores",
    );
    s.push_str("</svg>\n");
    s
}

fn main() {
    let out = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| root().join("docs/img"));
    std::fs::create_dir_all(&out).unwrap();
    for t in [&LIGHT, &DARK] {
        write(
            &out,
            &format!("salmon-cost-{}.svg", t.name),
            &salmon_figure(t),
        );
        write(&out, &format!("rsem-cores-{}.svg", t.name), &rsem_figure(t));
    }
}

fn write(dir: &Path, name: &str, body: &str) {
    std::fs::write(dir.join(name), body).unwrap();
}
