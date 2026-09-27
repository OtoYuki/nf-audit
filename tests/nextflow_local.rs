//! A real run, not a synthetic fixture: `testdata/nextflow-local/main.nf` run by Nextflow 25.04.8
//! on the local executor (no container, no cloud). Three `BUSY` tasks over-request CPUs, memory
//! and time; `ESCALATES` doubles its memory per attempt and one task exits 137 on its first
//! attempt. `scripts/e2e-nextflow.sh` regenerates these files and also applies the fragment in
//! Nextflow, checking that it loads and takes effect.

use nf_audit::analysis::{analyse, recommend, render_config, Rates};
use nf_audit::input::{merge, read_report_with_meta, read_trace};
use std::path::Path;

fn run() -> Vec<nf_audit::model::Task> {
    let d = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/nextflow-local");
    let trace = read_trace(&d.join("execution_trace.txt")).unwrap();
    let (report, meta) = read_report_with_meta(&d.join("execution_report.html")).unwrap();
    assert_eq!(meta.nextflow_version.as_deref(), Some("25.04.8"));
    merge(trace, report)
}

#[test]
fn local_executor_run_parses_and_merges() {
    let tasks = run();
    // 3 BUSY + 3 ESCALATES + the retried ESCALATES (s1).
    assert_eq!(tasks.len(), 7);
    let st = analyse(&tasks, &Rates::SEQERA_COMPUTE);
    assert_eq!(st.tasks_without_requests, 0);
    let esc = st
        .processes
        .iter()
        .find(|p| p.process == "NFCORE_DEMO:SUB:ESCALATES")
        .unwrap();
    assert_eq!(
        (
            esc.tasks,
            esc.retried_tasks,
            esc.failed_tasks,
            esc.killed_tasks
        ),
        (4, 1, 1, 1)
    );
    assert_eq!(
        (esc.base_mem_requested_gib, esc.retry_mem_requested_gib),
        (1.0, 2.0)
    );
    let tags: Vec<_> = st.tags.iter().filter_map(|g| g.tag.as_deref()).collect();
    assert_eq!(tags.len(), 3);
}

#[test]
fn fragment_lowers_busy_and_leaves_the_escalating_process_alone() {
    let st = analyse(&run(), &Rates::SEQERA_COMPUTE);
    let recs = recommend(&st, &Rates::SEQERA_COMPUTE, 1.25);
    let busy = recs
        .iter()
        .find(|r| r.process == "NFCORE_DEMO:SUB:BUSY")
        .unwrap();
    assert_eq!(
        (
            busy.cpus_now,
            busy.cpus_new,
            busy.mem_now_gib,
            busy.mem_new_gib
        ),
        (4.0, 2, 2.0, 1.0)
    );
    assert_eq!((busy.time_now_h, busy.time_new_h), (1.0, 0.25));
    assert!(recs
        .iter()
        .any(|r| r.process == "NFCORE_DEMO:SUB:ESCALATES" && r.needs_escalation));
    let cfg = render_config(&recs);
    assert!(cfg.contains(
        "    withName: 'NFCORE_DEMO:SUB:BUSY' {\n        cpus   = 2\n        memory = '1.GB'\n        time   = '15.m'\n    }"
    ));
    assert!(!cfg.contains("ESCALATES"));
}
