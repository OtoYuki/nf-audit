//! The README quotes nf-audit output and embeds generated figures. These checks keep the quotes
//! verbatim and the figure links live.

use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn readme() -> String {
    std::fs::read_to_string(root().join("README.md")).unwrap()
}

/// Every line inside `<!-- sample:<file> -->` … `<!-- /sample -->` appears verbatim in `<file>`.
#[test]
fn sample_output_is_quoted_verbatim() {
    let text = readme();
    let mut found = 0;
    for block in text.split("<!-- sample:").skip(1) {
        let file = block.split_whitespace().next().unwrap();
        let body = &block[..block.find("<!-- /sample -->").expect("closing marker")];
        let source = std::fs::read_to_string(root().join(file)).unwrap();
        let lines: Vec<&str> = source.lines().collect();
        for l in body.lines().skip(1) {
            if l.trim().is_empty() || l.starts_with("```") {
                continue;
            }
            assert!(lines.contains(&l), "README line not in {file}: {l}");
            found += 1;
        }
    }
    assert!(found >= 8, "sample block missing or empty");
}

/// Every image the README points at (`src="docs/img/…"` / `srcset="docs/img/…"`) exists.
#[test]
fn figures_exist() {
    let text = readme();
    let mut names = Vec::new();
    for attr in ["src=\"", "srcset=\""] {
        for (i, _) in text.match_indices(attr) {
            let value = &text[i + attr.len()..];
            let value = &value[..value.find('"').unwrap()];
            if let Some(name) = value.strip_prefix("docs/img/") {
                names.push(name.to_string());
            } else if let Some(name) = value.strip_prefix("docs/brand/") {
                assert!(root().join("docs/brand").join(name).is_file(), "{name}");
            }
        }
    }
    for name in &names {
        assert!(root().join("docs/img").join(name).is_file(), "{name}");
    }
    // Two figures, each in a light and a dark version.
    assert_eq!(names.len(), 4, "{names:?}");
}
