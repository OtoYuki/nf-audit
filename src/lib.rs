//! nf-audit: where the CPU-hours and dollars of a Nextflow run actually go.
//!
//! The library behind the `nf-audit` binary: parsers for Nextflow's trace TSV and execution
//! report ([`input`]), the task record ([`model`]), the cost model and right-sizing
//! ([`analysis`]) and the multi-run table ([`compare`]).

pub mod analysis;
pub mod compare;
pub mod input;
pub mod model;

/// The last `k` `:`-separated components of a process name (`A:B:C`, 2 -> `B:C`).
pub fn name_suffix(process: &str, k: usize) -> String {
    let parts: Vec<&str> = process.split(':').collect();
    parts[parts.len().saturating_sub(k)..].join(":")
}

/// For each process, the fewest trailing components that tell it apart from every other
/// process in `names` (1 = the bare last component).
pub fn unique_depths<'a>(names: &[&'a str]) -> std::collections::HashMap<&'a str, usize> {
    names
        .iter()
        .map(|&p| {
            let max = p.split(':').count();
            let k = (1..=max)
                .find(|&k| {
                    let mine = name_suffix(p, k);
                    names.iter().all(|&q| q == p || name_suffix(q, k) != mine)
                })
                .unwrap_or(max);
            (p, k)
        })
        .collect()
}

/// Short display name per process: the last path component, widened with parents until it is
/// unique (nf-core/rnaseq runs `SALMON_QUANT` under both `QUANTIFY_STAR_SALMON` and
/// `QUANTIFY_PSEUDO_ALIGNMENT`; sarek runs `CNVKIT_BATCH` under a germline and a somatic
/// `BAM_VARIANT_CALLING_CNVKIT`, which needs a third component).
pub fn display_names<'a>(
    processes: impl Iterator<Item = &'a str>,
) -> std::collections::HashMap<&'a str, String> {
    let all: Vec<&str> = processes.collect();
    unique_depths(&all)
        .into_iter()
        .map(|(p, k)| (p, name_suffix(p, k)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ambiguous_last_components_get_their_parent() {
        let n = display_names(["A:B:X", "A:C:X", "A:D:Y"].into_iter());
        assert_eq!(n["A:B:X"], "B:X");
        assert_eq!(n["A:C:X"], "C:X");
        assert_eq!(n["A:D:Y"], "Y");
        // The parent alone is not enough: germline and somatic share it.
        let n = display_names(["S:GERM:CNV:BATCH", "S:SOM:CNV:BATCH"].into_iter());
        assert_eq!(n["S:GERM:CNV:BATCH"], "GERM:CNV:BATCH");
        assert_eq!(n["S:SOM:CNV:BATCH"], "SOM:CNV:BATCH");
    }
}
