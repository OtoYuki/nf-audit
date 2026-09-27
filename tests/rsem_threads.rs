//! The figures `examples/rsem-threads/README.md` quotes, recomputed from the raw outputs
//! committed under `examples/rsem-threads/runs/`. Runs with a plain `cargo test`.

use std::collections::BTreeMap;
use std::path::Path;

fn runs(file: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/rsem-threads/runs")
        .join(file);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

struct Row {
    wall: f64,
    cpu_pct: f64,
    peak_bytes: f64,
}

fn summary() -> BTreeMap<u32, Row> {
    let text = runs("summary.tsv");
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().unwrap().split('\t').collect();
    assert_eq!(
        header,
        [
            "threads",
            "wall_s",
            "user_s",
            "sys_s",
            "cpu_pct",
            "peak_mem_bytes"
        ]
    );
    lines
        .map(|l| {
            let c: Vec<f64> = l.split('\t').map(|x| x.parse().unwrap()).collect();
            (
                c[0] as u32,
                Row {
                    wall: c[1],
                    cpu_pct: c[4],
                    peak_bytes: c[5],
                },
            )
        })
        .collect()
}

/// The results table: wall, speed-up, avg. CPU, CPUs × wall, `memory.peak` in decimal GB.
#[test]
fn results_table() {
    let s = summary();
    assert_eq!(s.keys().copied().collect::<Vec<_>>(), [1, 2, 4, 8, 12]);
    let base = s[&1].wall;
    let rows: Vec<String> = s
        .iter()
        .map(|(n, r)| {
            format!(
                "{n} {:.0} {:.2} {:.0} {:.0} {:.1}",
                r.wall,
                base / r.wall,
                r.cpu_pct,
                *n as f64 * r.wall,
                r.peak_bytes / 1e9
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            "1 1699 1.00 99 1699 2.7",
            "2 976 1.74 185 1952 5.2",
            "4 574 2.96 335 2297 3.5",
            "8 440 3.86 545 3522 4.8",
            "12 389 4.36 757 4673 5.2",
        ]
    );
    // "4 threads reach 68% of the 12-thread speed for 49% of the reserved CPU time".
    let speed = s[&12].wall / s[&4].wall;
    let cpu_time = (4.0 * s[&4].wall) / (12.0 * s[&12].wall);
    assert_eq!(
        ((speed * 100.0).round(), (cpu_time * 100.0).round()),
        (68.0, 49.0)
    );
}

/// Each `p*.time` is RSEM's own `S.time`; its "Estimating expression levels" line is the wall
/// time in the table, to the second.
#[test]
fn rsem_time_files_match_summary() {
    let s = summary();
    for (n, r) in &s {
        let t = runs(&format!("p{n}.time"));
        let secs: f64 = t
            .lines()
            .find_map(|l| l.strip_prefix("Estimating expression levels: "))
            .and_then(|x| x.strip_suffix(" s."))
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(secs, r.wall.round(), "p{n}");
    }
}

/// "`S.genes.results` and `S.isoforms.results` have the same sha256 at every thread count."
#[test]
fn outputs_identical_across_thread_counts() {
    let text = runs("hashes.txt");
    let mut by_file: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for l in text.lines() {
        let (hash, path) = l.split_once("  ").unwrap();
        let (dir, file) = path.split_once('/').unwrap();
        assert!(["p1", "p2", "p4", "p8", "p12"].contains(&dir), "{dir}");
        by_file.entry(file).or_default().push(hash);
    }
    assert_eq!(
        by_file.keys().copied().collect::<Vec<_>>(),
        ["S.genes.results", "S.isoforms.results"]
    );
    for (file, hashes) in by_file {
        assert_eq!(hashes.len(), 5, "{file}");
        assert!(hashes.windows(2).all(|w| w[0] == w[1]), "{file}");
    }
}

/// "`rsem-parse-alignments` at 1 CPU took 92.5–97.7 s over two runs for 18 M alignment entries
/// (~184–195 k entries/s)."
#[test]
fn parse_only_timing() {
    let text = runs("parse_only_p1.txt");
    let walls: Vec<f64> = text
        .lines()
        .filter_map(|l| l.strip_prefix("wall "))
        .map(|l| l.split_whitespace().next().unwrap().parse().unwrap())
        .collect();
    assert_eq!(walls.len(), 2);
    assert!(text.matches("Parsed 18000000 entries").count() == 2);
    let (lo, hi) = (walls[0].min(walls[1]), walls[0].max(walls[1]));
    assert_eq!(format!("{lo:.1} {hi:.1}"), "92.5 97.7");
    let rate = |w: f64| (18e6 / w / 1000.0).round();
    assert_eq!((rate(hi), rate(lo)), (184.0, 195.0));
}

/// I/O: "the task read 37.42 GB and wrote 4.52 GB for a 1.71 GB input BAM. Most of the reading
/// is `rsem-run-em`, 33.09 GB." And "21.9× the input size".
#[test]
fn io_volume() {
    let text = runs("io_p12.txt");
    let field = |line: &str, key: &str| -> f64 {
        let at = line.find(key).unwrap() + key.len();
        line[at..]
            .split_whitespace()
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    let lines: Vec<&str> = text.lines().collect();
    let total = lines.iter().find(|l| l.contains("rsem-  ")).unwrap();
    let em = lines.iter().find(|l| l.contains("rsem-run-em")).unwrap();
    assert_eq!(
        (field(total, "rchar"), field(total, "wchar")),
        (37.42, 4.52)
    );
    assert_eq!(field(em, "rchar"), 33.09);
    let bam: f64 = lines
        .iter()
        .find_map(|l| l.strip_prefix("input: "))
        .and_then(|l| l.split(", ").nth(1))
        .and_then(|l| l.strip_suffix(" bytes"))
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(format!("{:.2}", bam / 1e9), "1.71");
    assert_eq!(format!("{:.1}", 37.42e9 / bam), "21.9");
}

/// STAR: "96.55% of input pairs uniquely mapped on chr1", 7,792,588 read pairs.
#[test]
fn star_log() {
    let text = runs("star_chr1_log.txt");
    assert!(text.contains("Number of input reads |\t7792588"));
    assert!(text.contains("Uniquely mapped reads % |\t96.55%"));
}
