//! nf-audit: where the CPU-hours and dollars of a Nextflow run actually go.
//!
//! The library behind the `nf-audit` binary: parsers for Nextflow's trace TSV and execution
//! report ([`input`]), the task record ([`model`]), the cost model and right-sizing
//! ([`analysis`]) and the multi-run table ([`compare`]).

pub mod analysis;
pub mod compare;
pub mod input;
pub mod model;

/// Short display name per process: the last path component, widened to `PARENT:NAME` when
/// two processes share a last component (nf-core/rnaseq runs `SALMON_QUANT` under both
/// `QUANTIFY_STAR_SALMON` and `QUANTIFY_PSEUDO_ALIGNMENT`, for example).
pub fn display_names<'a>(
    processes: impl Iterator<Item = &'a str>,
) -> std::collections::HashMap<&'a str, String> {
    let all: Vec<&str> = processes.collect();
    let mut last_count: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for p in &all {
        *last_count
            .entry(p.rsplit(':').next().unwrap_or(p))
            .or_default() += 1;
    }
    all.into_iter()
        .map(|p| {
            let mut parts = p.rsplit(':');
            let last = parts.next().unwrap_or(p);
            let name = if last_count[last] > 1 {
                match parts.next() {
                    Some(parent) => format!("{parent}:{last}"),
                    None => last.to_string(),
                }
            } else {
                last.to_string()
            };
            (p, name)
        })
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
    }
}
