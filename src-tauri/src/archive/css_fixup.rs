use std::path::Path;
use crate::errors::Result;

/// Perform a filesystem-level search-and-replace on all CSS files within
/// the Elementor uploads directory.  This rewrites URLs embedded in the
/// pre-built CSS (background-image URLs, font URLs, etc.) so they point
/// at the local dev server instead of the original production domain.
///
/// The function handles three URL variants to match the logic used by
/// `search-replace.php` for the database:
///   1. Plain URL      — `https://example.com`
///   2. JSON-escaped   — `https:\/\/example.com`
///   3. URL-encoded    — `https%3A%2F%2Fexample.com`
pub async fn search_replace_in_css_files(
    extract_path: &Path,
    old_url: &str,
    new_url: &str,
) -> Result<()> {
    let css_dirs = [
        extract_path.join("wp-content").join("uploads").join("elementor").join("css"),
        extract_path.join("wp-content").join("uploads").join("elementor").join("google-fonts"),
    ];

    // Build the three search/replace pairs
    let old_json = old_url.replace('/', "\\/");
    let new_json = new_url.replace('/', "\\/");
    let old_urlenc = percent_encode(old_url);
    let new_urlenc = percent_encode(new_url);

    let pairs: Vec<(&str, &str)> = vec![
        (old_url, new_url),
        (&old_json, &new_json),
        (&old_urlenc, &new_urlenc),
    ];

    let extract_path_owned = extract_path.to_path_buf();

    for dir in css_dirs.iter() {
        if !dir.exists() {
            continue;
        }

        let mut files_processed = 0u32;
        let mut files_changed = 0u32;

        let entries = walkdir(dir)?;

        for entry in entries {
            if !entry.is_file() {
                continue;
            }
            // Process .css files (and .min.css etc.)
            let ext = entry.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "css" {
                continue;
            }

            files_processed += 1;

            let content = match tokio::fs::read_to_string(&entry).await {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("Could not read CSS file {}: {}", entry.display(), e);
                    continue;
                }
            };

            let mut new_content = content.clone();
            for (search, replace) in &pairs {
                if new_content.contains(*search) {
                    new_content = new_content.replace(*search, *replace);
                }
            }

            if new_content != content {
                if let Err(e) = tokio::fs::write(&entry, &new_content).await {
                    tracing::warn!("Could not write CSS file {}: {}", entry.display(), e);
                } else {
                    files_changed += 1;
                }
            }
        }

        let rel_dir = dir.strip_prefix(&extract_path_owned).unwrap_or(dir);
        tracing::info!(
            "CSS fixup in {}: {}/{} files updated",
            rel_dir.display(),
            files_changed,
            files_processed
        );
    }

    Ok(())
}

/// Recursively collect all file paths under `dir`.
fn walkdir(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut result = Vec::new();
    collect_files(dir, &mut result)?;
    Ok(result)
}

fn collect_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}

/// Simple percent-encoding matching PHP's `urlencode()` output.
/// Only encodes characters that PHP encodes — letters, digits, `-_.~` are kept.
fn percent_encode(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len() * 3);
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}
