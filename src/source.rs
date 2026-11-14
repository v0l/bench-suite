use crate::catalog::{SUITE, TOOLS, Tool};
use egui_bench::update::{Asset, Release, download, fetch_text, unpack};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const GITHUB: &str = "https://github.com";
const SUMS: &str = "SHA256SUMS";
const RETRY: &str = "retry";
const BUILD: &str = "cargo build --release";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Release,
    Archive,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kept {
    pub tag: String,
    pub file: PathBuf,
    pub bytes: u64,
    pub origin: Origin,
}

pub fn tarballs(home: &Path) -> PathBuf {
    home.join("source")
}

pub fn extracted(home: &Path, tag: &str) -> PathBuf {
    home.join("src").join(clean(tag))
}

fn clean(tag: &str) -> String {
    tag.replace(['/', '\\'], "_")
}

fn file_name(name: &str, tag: &str) -> String {
    format!("{name}-{}.tar.gz", clean(tag))
}

fn repo_name(repo: &str) -> &str {
    repo.rsplit('/').next().unwrap_or(repo)
}

fn marker(file: &Path) -> PathBuf {
    let name = file.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    file.with_file_name(format!("{name}.from"))
}

fn origin_of(file: &Path) -> Origin {
    match fs::read_to_string(marker(file)).map(|t| t.trim().to_string()).as_deref() {
        Ok("release") => Origin::Release,
        _ => Origin::Archive,
    }
}

pub fn kept(home: &Path, name: &str) -> Vec<Kept> {
    let Ok(entries) = fs::read_dir(tarballs(home)) else { return Vec::new() };
    let prefix = format!("{name}-");
    let mut out: Vec<Kept> = entries
        .flatten()
        .filter_map(|e| {
            let file = e.path();
            let base = file.file_name()?.to_str()?;
            let tag = base.strip_prefix(&prefix)?.strip_suffix(".tar.gz")?.to_string();
            let bytes = e.metadata().ok()?.len();
            Some(Kept { tag, origin: origin_of(&file), file, bytes })
        })
        .collect();
    out.sort_by(|a, b| a.tag.cmp(&b.tag));
    out
}

pub fn find(home: &Path, name: &str, tag: &str) -> Option<Kept> {
    kept(home, name).into_iter().find(|k| k.tag == clean(tag))
}

pub fn newest(home: &Path, name: &str) -> Option<Kept> {
    let mut all = kept(home, name);
    all.sort_by_key(|k| std::fs::metadata(&k.file).and_then(|m| m.modified()).ok());
    all.pop()
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut hash = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let temp = path.with_file_name(format!(".{name}.part"));
    fs::write(&temp, text).map_err(|e| format!("{}: {e}", temp.display()))?;
    let _ = fs::remove_file(path);
    fs::rename(&temp, path).map_err(|e| format!("{}: {e}", path.display()))
}

fn record_sum(dir: &Path, file: &str, sum: &str) -> Result<(), String> {
    let path = dir.join(SUMS);
    let old = fs::read_to_string(&path).unwrap_or_default();
    let mut lines: Vec<String> = old
        .lines()
        .filter(|l| l.split_whitespace().nth(1) != Some(file) && !l.trim().is_empty())
        .map(str::to_string)
        .collect();
    lines.push(format!("{sum}  {file}"));
    lines.sort_by(|a, b| a.split_whitespace().nth(1).cmp(&b.split_whitespace().nth(1)));
    write_atomic(&path, &(lines.join("\n") + "\n"))
}

#[cfg(test)]
pub fn checksums_match(home: &Path) -> bool {
    let dir = tarballs(home);
    let Ok(text) = fs::read_to_string(dir.join(SUMS)) else { return false };
    text.lines().filter(|l| !l.trim().is_empty()).all(|l| {
        let mut parts = l.split_whitespace();
        match (parts.next(), parts.next()) {
            (Some(sum), Some(file)) => {
                sha256_file(&dir.join(file)).is_ok_and(|have| have.eq_ignore_ascii_case(sum))
            }
            _ => false,
        }
    })
}

fn published_sum(release: &Release, asset: &str) -> Result<Option<String>, String> {
    let Some(sum) = release.asset(&format!("{asset}.sha256")) else { return Ok(None) };
    let text = fetch_text(&sum.url, 4096)?;
    let word = text.split_whitespace().next().unwrap_or_default().to_string();
    Ok((!word.is_empty()).then_some(word))
}

fn choose(name: &str, repo: &str, release: &Release, github: &str) -> (Asset, Origin) {
    let tag = &release.tag;
    let own = [name, repo_name(repo)]
        .iter()
        .find_map(|n| release.asset(&format!("{n}-{tag}-source.tar.gz")));
    match own {
        Some(asset) => (asset.clone(), Origin::Release),
        None => (
            Asset {
                name: file_name(name, tag),
                url: format!("{github}/{repo}/archive/refs/tags/{tag}.tar.gz"),
                bytes: 0,
                digest: None,
            },
            Origin::Archive,
        ),
    }
}

pub fn fetch(
    home: &Path,
    name: &str,
    repo: &str,
    release: &Release,
    github: &str,
) -> Result<Kept, String> {
    let dir = tarballs(home);
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let (asset, origin) = choose(name, repo, release, github);
    let part = dir.join(format!(".part-{}", std::process::id()));
    let _ = fs::remove_dir_all(&part);
    let stored = fetch_checked(&dir, &part, &asset, origin, release, name);
    let _ = fs::remove_dir_all(&part);
    stored
}

fn fetch_checked(
    dir: &Path,
    part: &Path,
    asset: &Asset,
    origin: Origin,
    release: &Release,
    name: &str,
) -> Result<Kept, String> {
    let got = download(asset, part, |_, _| {})?;
    let sum = sha256_file(&got)?;
    if let Some(want) = published_sum(release, &asset.name)?
        && !want.eq_ignore_ascii_case(&sum)
    {
        return Err(format!("{} does not match its published sha256", asset.name));
    }
    let file = file_name(name, &release.tag);
    let dest = dir.join(&file);
    let _ = fs::remove_file(&dest);
    fs::rename(&got, &dest).map_err(|e| format!("{}: {e}", dest.display()))?;
    let from = if origin == Origin::Release { "release" } else { "archive" };
    write_atomic(&marker(&dest), from)?;
    record_sum(dir, &file, &sum)?;
    let _ = fs::remove_file(dir.join(RETRY));
    let bytes = fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    Ok(Kept { tag: clean(&release.tag), file: dest, bytes, origin })
}

pub fn keep(home: &Path, tool: &Tool, release: &Release, github: &str) -> Option<String> {
    match fetch(home, tool.name, tool.repo, release, github) {
        Ok(_) => None,
        Err(e) => {
            let dir = tarballs(home);
            let _ = fs::create_dir_all(&dir);
            let _ = write_atomic(&dir.join(RETRY), &format!("{}\n{e}\n", release.tag));
            Some(format!("{}: could not keep the source for {}: {e}", tool.title, release.tag))
        }
    }
}

pub fn retry(home: &Path) -> Option<(String, String)> {
    let text = fs::read_to_string(tarballs(home).join(RETRY)).ok()?;
    let (tag, why) = text.split_once('\n')?;
    Some((tag.to_string(), why.trim().to_string()))
}

pub fn extract(home: &Path, name: &str, tag: &str) -> Result<PathBuf, String> {
    let kept = find(home, name, tag).ok_or_else(|| format!("no source is kept for {tag}"))?;
    let dest = extracted(home, tag);
    if dest.is_dir() && fs::read_dir(&dest).is_ok_and(|mut d| d.next().is_some()) {
        return Ok(dest);
    }
    let parent = dest.parent().ok_or("no folder")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = parent.join(format!(".{}.part", clean(tag)));
    let _ = fs::remove_dir_all(&temp);
    let _ = fs::remove_dir_all(&dest);
    let moved = unpack(&kept.file, &temp).and_then(|()| {
        let tops: Vec<PathBuf> =
            fs::read_dir(&temp).map_err(|e| e.to_string())?.flatten().map(|e| e.path()).collect();
        let root = match tops.as_slice() {
            [only] if only.is_dir() => only.clone(),
            _ => temp.clone(),
        };
        fs::rename(&root, &dest).map_err(|e| format!("{}: {e}", dest.display()))
    });
    let _ = fs::remove_dir_all(&temp);
    moved?;
    Ok(dest)
}

pub fn warnings(dir: &Path, origin: Origin) -> Vec<String> {
    let mut out = Vec::new();
    if dir.join(".gitmodules").is_file() && origin == Origin::Archive {
        out.push(
            "this repo uses git submodules and GitHub's tag archive leaves them empty, so it \
             will not build until they are added"
                .to_string(),
        );
    }
    let lfs = fs::read_to_string(dir.join(".gitattributes"))
        .is_ok_and(|t| t.lines().any(|l| l.contains("filter=lfs")));
    if lfs {
        out.push(
            "this repo keeps some files in git LFS, which a source archive holds only as \
             pointers"
                .to_string(),
        );
    }
    out
}

pub fn is_rust(dir: &Path) -> bool {
    dir.join("Cargo.toml").is_file()
}

pub fn is_vendored(dir: &Path) -> bool {
    dir.join("vendor").is_dir() && dir.join(".cargo").join("config.toml").is_file()
}

pub fn vendor(dir: &Path) -> Result<(), String> {
    if !is_rust(dir) {
        return Err("there is no Cargo.toml here, only Rust tools can be vendored".into());
    }
    let out = Command::new("cargo")
        .args(["vendor", "vendor"])
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("cargo would not start: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "cargo vendor failed: {}",
            String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or_default()
        ));
    }
    let config = String::from_utf8_lossy(&out.stdout).to_string();
    let cargo = dir.join(".cargo");
    fs::create_dir_all(&cargo).map_err(|e| e.to_string())?;
    write_atomic(&cargo.join("config.toml"), &config)
}

pub fn build_notes(dir: &Path) -> String {
    let mut out = String::new();
    if is_rust(dir) {
        out.push_str(BUILD);
        if is_vendored(dir) {
            out.push_str("  (dependencies are vendored, no network needed)");
        }
        out.push('\n');
    }
    let readme = ["README.md", "README", "readme.md"]
        .iter()
        .find_map(|n| fs::read_to_string(dir.join(n)).ok())
        .unwrap_or_default();
    let mut lines = readme.lines();
    let mut section: Vec<&str> = Vec::new();
    while let Some(line) = lines.next() {
        if line.starts_with('#') && line.to_lowercase().contains("build") {
            for next in lines.by_ref() {
                if next.starts_with('#') || section.len() >= 14 {
                    break;
                }
                section.push(next);
            }
            break;
        }
    }
    let notes = section.join("\n");
    let notes = notes.trim();
    if !notes.is_empty() {
        out.push_str("\nFrom the README:\n");
        out.push_str(notes);
        out.push('\n');
    }
    out
}

pub fn dir_size(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else { return 0 };
    if meta.is_file() {
        return meta.len();
    }
    if !meta.is_dir() {
        return 0;
    }
    fs::read_dir(path).map(|d| d.flatten().map(|e| dir_size(&e.path())).sum()).unwrap_or(0)
}

pub fn used(home: &Path) -> u64 {
    dir_size(&tarballs(home)) + dir_size(&home.join("src"))
}

pub fn total_used() -> u64 {
    TOOLS.iter().chain(std::iter::once(&SUITE)).map(|t| used(&crate::install::home(t))).sum()
}

pub fn discard_home(home: &Path, keep_source: bool) -> Result<(), String> {
    let Ok(entries) = fs::read_dir(home) else { return Ok(()) };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if keep_source && (name == "source" || name == "src") {
            continue;
        }
        let path = entry.path();
        let gone = if path.is_dir() && !path.is_symlink() {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
        gone.map_err(|e| format!("{}: {e}", path.display()))?;
    }
    if !keep_source {
        let _ = fs::remove_dir(home);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    struct Server {
        base: String,
        hits: Arc<Mutex<Vec<String>>>,
    }

    fn serve(routes: HashMap<String, Vec<u8>>) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let hits = Arc::new(Mutex::new(Vec::new()));
        let seen = hits.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut stream = stream;
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).unwrap_or(0);
                let head = String::from_utf8_lossy(&buf[..n]).to_string();
                let path = head.split_whitespace().nth(1).unwrap_or("/").to_string();
                seen.lock().unwrap().push(path.clone());
                let reply = match routes.get(&path) {
                    Some(body) => {
                        let mut r = format!(
                            "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                            body.len()
                        )
                        .into_bytes();
                        r.extend_from_slice(body);
                        r
                    }
                    None => {
                        b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
                            .to_vec()
                    }
                };
                let _ = stream.write_all(&reply);
            }
        });
        Server { base, hits }
    }

    fn tarball(top: &str, files: &[(&str, &str)]) -> Vec<u8> {
        let gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        let mut tar = tar::Builder::new(gz);
        for (path, body) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(body.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, format!("{top}/{path}"), body.as_bytes()).unwrap();
        }
        tar.into_inner().unwrap().finish().unwrap()
    }

    fn sum(bytes: &[u8]) -> String {
        Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
    }

    fn home(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bench-source-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn release(server: &Server, tag: &str, assets: &[(&str, &str)]) -> Release {
        Release {
            version: tag.trim_start_matches('v').to_string(),
            tag: tag.to_string(),
            page: String::new(),
            published: String::new(),
            assets: assets
                .iter()
                .map(|(name, path)| Asset {
                    name: name.to_string(),
                    url: format!("{}{path}", server.base),
                    bytes: 0,
                    digest: None,
                })
                .collect(),
        }
    }

    fn tool() -> &'static Tool {
        TOOLS.iter().find(|t| t.name == "gcad").unwrap()
    }

    #[test]
    fn the_release_source_asset_is_kept_with_a_matching_checksum() {
        let body =
            tarball("gcad-v1.0.0", &[("Cargo.toml", "[package]"), ("a/b.rs", "fn main(){}")]);
        let mut routes = HashMap::new();
        routes.insert("/dl/src".to_string(), body.clone());
        routes.insert(
            "/dl/sum".to_string(),
            format!("{}  gcad-v1.0.0-source.tar.gz\n", sum(&body)).into_bytes(),
        );
        let server = serve(routes);
        let rel = release(
            &server,
            "v1.0.0",
            &[
                ("gcad-v1.0.0-source.tar.gz", "/dl/src"),
                ("gcad-v1.0.0-source.tar.gz.sha256", "/dl/sum"),
            ],
        );
        let home = home("asset");
        let got = fetch(&home, "gcad", "v0l/gcad", &rel, &server.base).unwrap();
        assert_eq!(got.file, home.join("source/gcad-v1.0.0.tar.gz"));
        assert_eq!(got.origin, Origin::Release);
        assert_eq!(fs::read(&got.file).unwrap(), body);
        let sums = fs::read_to_string(home.join("source/SHA256SUMS")).unwrap();
        assert_eq!(sums, format!("{}  gcad-v1.0.0.tar.gz\n", sum(&body)));
        assert!(checksums_match(&home));
        assert!(!server.hits.lock().unwrap().iter().any(|p| p.contains("archive")));
        let left: Vec<_> = fs::read_dir(home.join("source")).unwrap().flatten().collect();
        assert!(left.iter().all(|e| !e.file_name().to_string_lossy().contains("part")));
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn a_source_that_fails_its_published_checksum_is_not_kept() {
        let body = tarball("gcad-v1.0.0", &[("Cargo.toml", "x")]);
        let mut routes = HashMap::new();
        routes.insert("/dl/src".to_string(), body);
        routes.insert("/dl/sum".to_string(), format!("{}  x\n", "0".repeat(64)).into_bytes());
        let server = serve(routes);
        let rel = release(
            &server,
            "v1.0.0",
            &[
                ("gcad-v1.0.0-source.tar.gz", "/dl/src"),
                ("gcad-v1.0.0-source.tar.gz.sha256", "/dl/sum"),
            ],
        );
        let home = home("badsum");
        let err = fetch(&home, "gcad", "v0l/gcad", &rel, &server.base).unwrap_err();
        assert!(err.contains("sha256"), "{err}");
        assert!(kept(&home, "gcad").is_empty());
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn the_tag_archive_is_used_when_the_release_has_no_source_asset() {
        let body = tarball("gcad-abc123", &[("Cargo.toml", "[package]")]);
        let mut routes = HashMap::new();
        routes.insert("/v0l/gcad/archive/refs/tags/v1.0.0.tar.gz".to_string(), body.clone());
        let server = serve(routes);
        let rel = release(&server, "v1.0.0", &[]);
        let home = home("archive");
        let got = fetch(&home, "gcad", "v0l/gcad", &rel, &server.base).unwrap();
        assert_eq!(got.origin, Origin::Archive);
        assert_eq!(fs::read(&got.file).unwrap(), body);
        assert!(checksums_match(&home));
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn a_failed_source_download_is_recorded_for_retry_and_leaves_no_file() {
        let server = serve(HashMap::new());
        let rel = release(&server, "v1.0.0", &[]);
        let home = home("fail");
        let warning = keep(&home, tool(), &rel, &server.base).unwrap();
        assert!(warning.contains("could not keep the source"), "{warning}");
        assert_eq!(retry(&home).unwrap().0, "v1.0.0");
        assert!(kept(&home, "gcad").is_empty());
        let names: Vec<String> = fs::read_dir(home.join("source"))
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["retry"]);

        let mut routes = HashMap::new();
        routes.insert(
            "/v0l/gcad/archive/refs/tags/v1.0.0.tar.gz".to_string(),
            tarball("gcad-1", &[("Cargo.toml", "x")]),
        );
        let working = serve(routes);
        assert!(keep(&home, tool(), &rel, &working.base).is_none());
        assert!(retry(&home).is_none());
        assert_eq!(kept(&home, "gcad").len(), 1);
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn extracting_unpacks_to_the_versioned_folder_and_keeps_the_older_one() {
        let mut routes = HashMap::new();
        for (tag, text) in [("v1.0.0", "one"), ("v1.1.0", "two")] {
            routes.insert(
                format!("/v0l/gcad/archive/refs/tags/{tag}.tar.gz"),
                tarball(
                    &format!("gcad-{tag}"),
                    &[("Cargo.toml", "[package]"), ("src/main.rs", text)],
                ),
            );
        }
        let server = serve(routes);
        let home = home("extract");
        for tag in ["v1.0.0", "v1.1.0"] {
            fetch(&home, "gcad", "v0l/gcad", &release(&server, tag, &[]), &server.base).unwrap();
        }
        let first = extract(&home, "gcad", "v1.0.0").unwrap();
        assert_eq!(first, home.join("src/v1.0.0"));
        assert_eq!(fs::read_to_string(first.join("src/main.rs")).unwrap(), "one");
        let second = extract(&home, "gcad", "v1.1.0").unwrap();
        assert_eq!(fs::read_to_string(second.join("src/main.rs")).unwrap(), "two");
        assert_eq!(fs::read_to_string(first.join("src/main.rs")).unwrap(), "one");
        assert_eq!(kept(&home, "gcad").len(), 2);
        assert!(extract(&home, "gcad", "v9.9.9").is_err());
        assert!(used(&home) > 0);
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn removing_a_tool_keeps_or_drops_its_source_as_asked() {
        for keep_source in [true, false] {
            let home = home(if keep_source { "keep" } else { "drop" });
            fs::create_dir_all(home.join("source")).unwrap();
            fs::create_dir_all(home.join("src/v1")).unwrap();
            fs::write(home.join("source/gcad-v1.tar.gz"), "x").unwrap();
            fs::write(home.join("gcad"), "bin").unwrap();
            fs::write(home.join("lib.so"), "lib").unwrap();
            discard_home(&home, keep_source).unwrap();
            assert!(!home.join("gcad").exists());
            assert!(!home.join("lib.so").exists());
            assert_eq!(home.join("source/gcad-v1.tar.gz").exists(), keep_source);
            assert_eq!(home.join("src/v1").exists(), keep_source);
            assert_eq!(home.exists(), keep_source);
            let _ = fs::remove_dir_all(&home);
        }
    }

    #[test]
    fn submodules_and_lfs_are_warned_about() {
        let dir = home("warn");
        fs::write(dir.join(".gitmodules"), "[submodule \"x\"]").unwrap();
        fs::write(dir.join(".gitattributes"), "*.bin filter=lfs diff=lfs merge=lfs").unwrap();
        assert_eq!(warnings(&dir, Origin::Archive).len(), 2);
        assert_eq!(warnings(&dir, Origin::Release).len(), 1);
        fs::remove_file(dir.join(".gitattributes")).unwrap();
        fs::remove_file(dir.join(".gitmodules")).unwrap();
        assert!(warnings(&dir, Origin::Archive).is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn build_notes_name_cargo_and_quote_the_readme_build_section() {
        let dir = home("notes");
        fs::write(dir.join("Cargo.toml"), "[package]").unwrap();
        fs::write(
            dir.join("README.md"),
            "# Tool\nintro\n## Building\nneeds libx11\nrun make\n## Usage\nno\n",
        )
        .unwrap();
        let notes = build_notes(&dir);
        assert!(notes.starts_with("cargo build --release"));
        assert!(notes.contains("needs libx11"));
        assert!(!notes.contains("no\n"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    #[ignore = "downloads a small crate's tag archive, vendors it and builds it offline"]
    fn a_real_tag_archive_vendors_and_builds_offline() {
        let repo = std::env::var("BENCH_SOURCE_REPO").unwrap_or("sharkdp/hexyl".into());
        let tag = std::env::var("BENCH_SOURCE_TAG").unwrap_or("v0.16.0".into());
        let name = repo_name(&repo).to_string();
        let rel = Release {
            version: tag.trim_start_matches('v').into(),
            tag: tag.clone(),
            page: String::new(),
            published: String::new(),
            assets: Vec::new(),
        };
        let home = home("real");
        let started = std::time::Instant::now();
        let got = fetch(&home, &name, &repo, &rel, GITHUB).unwrap();
        let fetched = started.elapsed();
        let dir = extract(&home, &name, &tag).unwrap();
        assert!(is_rust(&dir));
        let t = std::time::Instant::now();
        vendor(&dir).unwrap();
        let vendored = t.elapsed();
        assert!(is_vendored(&dir));
        let t = std::time::Instant::now();
        let status = Command::new("cargo")
            .args(["build", "--release", "--offline"])
            .env("CARGO_NET_OFFLINE", "true")
            .env("CARGO_BUILD_JOBS", "4")
            .env("CARGO_HOME", home.join("empty-cargo-home"))
            .current_dir(&dir)
            .status()
            .unwrap();
        assert!(status.success());
        println!(
            "tarball {} bytes, fetched in {fetched:?}, vendored in {vendored:?} ({} bytes), offline build {:?}, tree {} bytes",
            got.bytes,
            dir_size(&dir.join("vendor")),
            t.elapsed(),
            dir_size(&dir)
        );
        fs::remove_dir_all(&home).unwrap();
    }
}
