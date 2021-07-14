//! This module greps parser's code for specially formatted comments and turns
//! them into tests.

use std::{
    fs, iter,
    path::{Path, PathBuf},
};

use rustc_hash::FxHashMap;

#[test]
fn sourcegen_parser_tests() {
    let grammar_dir = sourcegen::project_root().join(Path::new("crates/parser/src/grammar"));
    let tests = tests_from_dir(&grammar_dir);

    install_tests(&tests.ok, "crates/syntax/test_data/parser/inline/ok");
    install_tests(&tests.err, "crates/syntax/test_data/parser/inline/err");

    fn install_tests(tests: &FxHashMap<String, Test>, into: &str) {
        let tests_dir = sourcegen::project_root().join(into);
        if !tests_dir.is_dir() {
            fs::create_dir_all(&tests_dir).unwrap();
        }
        // ok is never actually read, but it needs to be specified to create a Test in existing_tests
        let existing = existing_tests(&tests_dir, true);
        for t in existing.keys().filter(|&t| !tests.contains_key(t)) {
            panic!("Test is deleted: {}", t);
        }

        let mut new_idx = existing.len() + 1;
        for (name, test) in tests {
            let path = match existing.get(name) {
                Some((path, _test)) => path.clone(),
                None => {
                    let file_name = format!("{:04}_{}.rs", new_idx, name);
                    new_idx += 1;
                    tests_dir.join(file_name)
                }
            };
            sourcegen::ensure_file_contents(&path, &test.text);
        }
    }
}

#[derive(Debug)]
struct Test {
    name: String,
    text: String,
    ok: bool,
}

#[derive(Default, Debug)]
struct Tests {
    ok: FxHashMap<String, Test>,
    err: FxHashMap<String, Test>,
}

fn collect_tests(s: &str) -> Vec<Test> {
    let mut res = Vec::new();
    for comment_block in sourcegen::CommentBlock::extract_untagged(s) {
        let first_line = &comment_block.contents[0];
        let (name, ok) = if let Some(name) = first_line.strip_prefix("test ") {
            (name.to_string(), true)
        } else if let Some(name) = first_line.strip_prefix("test_err ") {
            (name.to_string(), false)
        } else {
            continue;
        };
        let text: String = comment_block.contents[1..]
            .iter()
            .cloned()
            .chain(iter::once(String::new()))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!text.trim().is_empty() && text.ends_with('\n'));
        res.push(Test { name, text, ok })
    }
    res
}

fn tests_from_dir(dir: &Path) -> Tests {
    let mut res = Tests::default();
    for entry in sourcegen::list_rust_files(dir) {
        process_file(&mut res, entry.as_path());
    }
    let grammar_rs = dir.parent().unwrap().join("grammar.rs");
    process_file(&mut res, &grammar_rs);
    return res;

    fn process_file(res: &mut Tests, path: &Path) {
        let text = fs::read_to_string(path).unwrap();

        for test in collect_tests(&text) {
            if test.ok {
                if let Some(old_test) = res.ok.insert(test.name.clone(), test) {
                    panic!("Duplicate test: {}", old_test.name);
                }
            } else if let Some(old_test) = res.err.insert(test.name.clone(), test) {
                panic!("Duplicate test: {}", old_test.name);
            }
        }
    }
}

fn existing_tests(dir: &Path, ok: bool) -> FxHashMap<String, (PathBuf, Test)> {
    let mut res = FxHashMap::default();
    for file in fs::read_dir(dir).unwrap() {
        let file = file.unwrap();
        let path = file.path();
        if path.extension().unwrap_or_default() != "rs" {
            continue;
        }
        let name = {
            let file_name = path.file_name().unwrap().to_str().unwrap();
            file_name[5..file_name.len() - 3].to_string()
        };
        let text = fs::read_to_string(&path).unwrap();
        let test = Test { name: name.clone(), text, ok };
        if let Some(old) = res.insert(name, (path, test)) {
            println!("Duplicate test: {:?}", old);
        }
    }
    res
}