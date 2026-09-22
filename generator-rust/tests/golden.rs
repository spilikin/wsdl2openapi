//! Generates every fixture and compares the output with the checked-in tree
//! under `tests/fixtures/<name>/expected`. Fixture inputs come from the
//! Kotlin generator's fixtures, so both generators are pinned by the same
//! documents; Rust-only fixtures carry their own `api.json`/`naming.json`.
//!
//! Run with `UPDATE_GOLDEN=1` to rewrite the expected trees.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use wsdl2openapi2rust::model::Api;
use wsdl2openapi2rust::naming::NamingStrategy;
use wsdl2openapi2rust::{Options, generate, write_files};

const KOTLIN_FIXTURES: &str = "../generator-kotlin/app/src/test/resources/fixtures";
const RUST_FIXTURES: &str = "tests/fixtures";

fn fixture_inputs() -> BTreeMap<String, PathBuf> {
    let mut inputs = BTreeMap::new();
    for dir in [KOTLIN_FIXTURES, RUST_FIXTURES] {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
        for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let path = entry.unwrap().path();
            if path.join("api.json").is_file() {
                inputs.insert(
                    path.file_name().unwrap().to_string_lossy().into_owned(),
                    path,
                );
            }
        }
    }
    inputs
}

fn read_tree(dir: &Path) -> BTreeMap<PathBuf, String> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, String>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let contents = fs::read_to_string(&path).unwrap();
                out.insert(path.strip_prefix(root).unwrap().to_owned(), contents);
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

#[test]
fn generated_trees_match_expected() {
    let update = std::env::var_os("UPDATE_GOLDEN").is_some();
    let mut failures = Vec::new();

    for (name, input) in fixture_inputs() {
        let api: Api =
            serde_json::from_str(&fs::read_to_string(input.join("api.json")).unwrap()).unwrap();
        let naming = match fs::read_to_string(input.join("naming.json")) {
            Ok(json) => NamingStrategy::from_json(&json).unwrap(),
            Err(_) => NamingStrategy::default(),
        };
        let files =
            generate(api, &naming, &Options::default()).unwrap_or_else(|e| panic!("{name}: {e:#}"));
        let expected_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(RUST_FIXTURES)
            .join(&name)
            .join("expected");

        if update {
            let _ = fs::remove_dir_all(&expected_dir);
            write_files(&expected_dir, &files).unwrap();
            continue;
        }

        let actual: BTreeMap<PathBuf, String> =
            files.into_iter().map(|f| (f.path, f.contents)).collect();
        let expected = read_tree(&expected_dir);
        if actual.keys().ne(expected.keys()) {
            failures.push(format!(
                "{name}: files differ\n  expected {:?}\n  actual   {:?}",
                expected.keys().collect::<Vec<_>>(),
                actual.keys().collect::<Vec<_>>()
            ));
            continue;
        }
        for (path, contents) in &actual {
            if expected[path] != *contents {
                failures.push(format!(
                    "{name}/{}: contents differ\n{contents}",
                    path.display()
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "rerun with UPDATE_GOLDEN=1 if intended:\n{}",
        failures.join("\n")
    );
}
