//! Round-trip and invariant test cases for the TOML storage layer.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::model::{Environment, Folder, HttpRequest, Node, Workspace, WsRequest};
use crate::storage::cache::{self, IndexCache};
use crate::storage::error::StorageResult;
use crate::storage::{
    build_index, ensure_gitignore, load_workspace, save_workspace, workspace_from_dataset,
};

use super::fixtures::{read_tree, sample, temp_dir};

#[test]
fn round_trip_equals_original() -> StorageResult<()> {
    let dir = temp_dir("roundtrip");
    let ws = sample();
    save_workspace(&dir, &ws)?;
    let loaded = load_workspace(&dir)?;
    assert_eq!(loaded, ws, "loaded workspace must equal the original");
    let _ = fs::remove_dir_all(&dir);
    Ok(())
}

/// Empty names must NOT be normalized to the slug on load (that would break round-trip and
/// byte-stability). The slug fallback lives at display time instead.
#[test]
fn empty_names_round_trip_and_stay_stable() -> StorageResult<()> {
    let dir = temp_dir("empty-names");
    let mut ws = Workspace::default_with_name(String::new());
    ws.environments = vec![Environment {
        slug: "e".to_string(),
        name: String::new(),
        variables: Vec::new(),
    }];
    ws.root = vec![Node::Folder(Folder {
        children: vec![
            Node::Http(HttpRequest::new("anon-req", "", "GET", "https://x/a")),
            Node::Ws(WsRequest::new("anon-ws", "", "wss://x/w")),
        ],
        ..Folder::new("anon-folder", "")
    })];

    save_workspace(&dir, &ws)?;
    let loaded = load_workspace(&dir)?;
    assert_eq!(
        loaded, ws,
        "empty names must round-trip exactly, not become slugs"
    );

    let mut snap1 = BTreeMap::new();
    read_tree(&dir, &dir, &mut snap1);
    save_workspace(&dir, &loaded)?;
    let mut snap2 = BTreeMap::new();
    read_tree(&dir, &dir, &mut snap2);
    assert_eq!(
        snap1, snap2,
        "empty-name workspace must re-save byte-stable"
    );

    let folder_node = &loaded.root[0];
    assert_eq!(
        folder_node.display_name(),
        "anon-folder",
        "display falls back to slug"
    );
    if let Node::Folder(f) = folder_node {
        assert_eq!(f.children[0].display_name(), "anon-req");
    }
    let _ = fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn save_is_idempotent_stable_bytes() -> StorageResult<()> {
    let dir = temp_dir("stable");
    let ws = sample();
    save_workspace(&dir, &ws)?;

    let mut snap1 = BTreeMap::new();
    read_tree(&dir, &dir, &mut snap1);

    let loaded = load_workspace(&dir)?;
    save_workspace(&dir, &loaded)?;

    let mut snap2 = BTreeMap::new();
    read_tree(&dir, &dir, &mut snap2);

    assert_eq!(
        snap1, snap2,
        "re-saving an unchanged workspace must not change any file"
    );
    assert!(snap1.contains_key("sasin.toml"));
    let _ = fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn order_is_preserved_non_lexically() -> StorageResult<()> {
    let dir = temp_dir("order");
    save_workspace(&dir, &sample())?;
    let loaded = load_workspace(&dir)?;

    let api = loaded
        .root
        .iter()
        .find_map(|n| match n {
            Node::Folder(f) if f.slug == "api" => Some(f),
            _ => None,
        })
        .expect("api folder present");
    let slugs: Vec<&str> = api.children.iter().map(|n| n.slug()).collect();
    assert_eq!(
        slugs,
        vec!["upload", "form", "raw-bin", "gql", "zebra", "alpha"]
    );
    let _ = fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn gitignore_excludes_cache_once() -> StorageResult<()> {
    let dir = temp_dir("gitignore");
    fs::create_dir_all(&dir).ok();
    ensure_gitignore(&dir)?;
    ensure_gitignore(&dir)?;
    let gi = fs::read_to_string(dir.join(".gitignore")).unwrap_or_default();
    assert_eq!(gi.matches(".sasin-cache").count(), 1);
    let _ = fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn index_cache_round_trips() -> StorageResult<()> {
    let dir = temp_dir("index");
    fs::create_dir_all(&dir).ok();
    let ws = sample();
    let index = build_index(Path::new("/tmp/sample-ws"), &ws);
    assert!(
        index
            .entries
            .iter()
            .any(|e| e.path == "auth/login" && e.method == "POST")
    );
    assert!(index.entries.iter().any(|e| e.path == "chat"));

    let path = dir.join("index.bc.zst");
    cache::write_cache(&path, &index)?;
    let back = cache::read_cache::<IndexCache>(&path);
    assert_eq!(back.as_ref(), Some(&index));
    let _ = fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn legacy_dataset_migrates_to_tree() -> StorageResult<()> {
    use crate::models::HttpMethod;
    use crate::persist::{Collection, Dataset, Request};

    let mut ds = Dataset::default();
    ds.collections.push(Collection {
        id: 1,
        name: "My Coll".to_string(),
        requests: vec![Request::new(
            2,
            "Get Thing",
            HttpMethod::Get,
            "https://x/thing",
        )],
    });

    let ws = workspace_from_dataset(&ds, "Imported");
    let dir = temp_dir("migrate");
    save_workspace(&dir, &ws)?;
    let loaded = load_workspace(&dir)?;

    assert_eq!(loaded.root.len(), 1);
    match &loaded.root[0] {
        Node::Folder(f) => {
            assert_eq!(f.slug, "my-coll");
            assert_eq!(f.children.len(), 1);
            match &f.children[0] {
                Node::Http(r) => {
                    assert_eq!(r.slug, "get-thing");
                    assert_eq!(r.method, "GET");
                    assert_eq!(r.url, "https://x/thing");
                }
                _ => panic!("expected http request"),
            }
        }
        _ => panic!("expected folder"),
    }
    let _ = fs::remove_dir_all(&dir);
    Ok(())
}
