use prmpt::{InjectOperation, Injector};
use std::fs;
use tempfile::tempdir;

#[test]
fn inject_plain_path() {
    let dir = tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("src")).unwrap();
    fs::write(repo.join("src/lib.rs"), "fn old() {}\n").unwrap();

    let input = repo.join("input.in");
    fs::write(&input, "src/lib.rs\n```rust\nfn new_fn() {}\n```\n").unwrap();

    let injector = Injector;
    injector.inject(&input, repo).unwrap();

    let contents = fs::read_to_string(repo.join("src/lib.rs")).unwrap();
    assert!(contents.contains("new_fn"));
}

#[test]
fn inject_backticked_path() {
    let dir = tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("src")).unwrap();
    fs::write(repo.join("src/lib.rs"), "fn start() {}\n").unwrap();

    let input = repo.join("input.in");
    fs::write(&input, "### `src/lib.rs`\n```rust\nfn added() {}\n```\n").unwrap();

    let injector = Injector;
    injector.inject(&input, repo).unwrap();

    let contents = fs::read_to_string(repo.join("src/lib.rs")).unwrap();
    assert!(contents.contains("added"));
}

#[test]
fn inject_fence_with_path() {
    let dir = tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("src")).unwrap();
    fs::write(repo.join("src/lib.rs"), "fn legacy() {}\n").unwrap();

    let input = repo.join("input.in");
    fs::write(&input, "```src/lib.rs\nfn replaced() {}\n```\n").unwrap();

    let injector = Injector;
    injector.inject(&input, repo).unwrap();

    let contents = fs::read_to_string(repo.join("src/lib.rs")).unwrap();
    assert!(contents.contains("replaced"));
}

#[test]
fn inject_fence_with_language_and_path() {
    let dir = tempdir().unwrap();
    let repo = dir.path();
    fs::create_dir_all(repo.join("src")).unwrap();
    fs::write(repo.join("src/lib.rs"), "fn pre() {}\n").unwrap();

    let input = repo.join("input.in");
    fs::write(&input, "```rust src/lib.rs\nfn update() {}\n```\n").unwrap();

    let injector = Injector;
    injector.inject(&input, repo).unwrap();

    let contents = fs::read_to_string(repo.join("src/lib.rs")).unwrap();
    assert!(contents.contains("update"));
}
