use crate::model::{Argument, PromptData};
use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn get_folder_path(
    folder: Option<&str>,
    git_url: Option<&str>,
    cache_dir: &str,
    auto_pull: bool,
) -> Result<PathBuf> {
    if let Some(url) = git_url {
        let repo_path = get_cache_path(url, cache_dir)?;
        clone_or_update(&repo_path, url, auto_pull)?;
        Ok(if let Some(f) = folder {
            repo_path.join(f)
        } else {
            repo_path
        })
    } else {
        let path =
            folder.ok_or_else(|| anyhow::anyhow!("Either folder or git-url must be provided"))?;
        let expanded = shellexpand::tilde(path);
        Ok(std::env::current_dir()?.join(expanded.as_ref()))
    }
}

fn get_cache_path(git_url: &str, cache_dir: &str) -> Result<PathBuf> {
    let (owner, name) = parse_git_url(git_url)?;
    let expanded = shellexpand::tilde(cache_dir);
    Ok(PathBuf::from(expanded.as_ref())
        .join("git")
        .join(owner)
        .join(name))
}

fn parse_git_url(git_url: &str) -> Result<(String, String)> {
    // Handle SSH URLs: git@github.com:user/repo.git
    if let Some(ssh_part) = git_url.strip_prefix("git@") {
        if let Some(colon_pos) = ssh_part.find(':') {
            let path = &ssh_part[colon_pos + 1..];
            let parts: Vec<&str> = path.trim_end_matches(".git").split('/').collect();
            if parts.len() >= 2 {
                return Ok((
                    parts[parts.len() - 2].to_string(),
                    parts[parts.len() - 1].to_string(),
                ));
            }
        }
    }

    // Handle HTTPS URLs
    let url = url::Url::parse(git_url)?;
    let path = url.path().trim_start_matches('/').trim_end_matches(".git");
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 2 {
        Ok((
            parts[parts.len() - 2].to_string(),
            parts[parts.len() - 1].to_string(),
        ))
    } else {
        anyhow::bail!("Cannot extract user/repo from git URL: {}", git_url)
    }
}

fn clone_or_update(path: &Path, url: &str, auto_pull: bool) -> Result<()> {
    // Setup SSH credential callback to use ssh-agent for authentication
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        // Try to authenticate using SSH keys from ssh-agent
        // Falls back to "git" username if not specified in URL
        git2::Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
    });

    if path.exists() {
        if auto_pull {
            let repo = git2::Repository::open(path)?;
            let mut remote = repo.find_remote("origin")?;

            // Configure fetch options with SSH credentials
            let mut fo = git2::FetchOptions::new();
            fo.remote_callbacks(callbacks);
            remote.fetch(&[] as &[&str], Some(&mut fo), None)?;

            let fetch_head = repo.find_reference("FETCH_HEAD")?;
            let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
            let analysis = repo.merge_analysis(&[&fetch_commit])?;

            // Fast-forward if possible
            if analysis.0.is_fast_forward() {
                let head = repo.head()?;
                let refname = head
                    .name()
                    .ok_or_else(|| anyhow::anyhow!("Invalid HEAD reference"))?;
                let mut reference = repo.find_reference(refname)?;
                reference.set_target(fetch_commit.id(), "Fast-Forward")?;
                repo.set_head(refname)?;
                repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
            }
        }
    } else {
        // Clone repository with shallow depth and SSH credentials
        std::fs::create_dir_all(path.parent().unwrap())?;
        let mut builder = git2::build::RepoBuilder::new();
        let mut fo = git2::FetchOptions::new();
        fo.remote_callbacks(callbacks);
        fo.depth(1); // Shallow clone to save bandwidth
        builder.fetch_options(fo);
        builder.clone(url, path)?;
    }
    Ok(())
}

pub fn scan_markdown_files(folder: &Path, skip_frontmatter: bool) -> Result<Vec<PromptData>> {
    if !folder.exists() || !folder.is_dir() {
        eprintln!(
            "Warning: folder path '{}' does not exist or is not a directory",
            folder.display()
        );
        return Ok(Vec::new());
    }

    let mut prompts = Vec::new();
    for entry in WalkDir::new(folder).into_iter().filter_map(|e| e.ok()) {
        if entry.path().extension().and_then(|s| s.to_str()) == Some("md") {
            match std::fs::read_to_string(entry.path()) {
                Ok(content) => {
                    match parse_markdown(entry.path(), folder, &content, skip_frontmatter) {
                        Ok(prompt) => prompts.push(prompt),
                        Err(e) => eprintln!(
                            "Warning: failed to process {}: {}",
                            entry.path().display(),
                            e
                        ),
                    }
                }
                Err(e) => eprintln!("Warning: failed to read {}: {}", entry.path().display(), e),
            }
        }
    }
    Ok(prompts)
}

fn parse_markdown(
    file: &Path,
    folder: &Path,
    content: &str,
    skip_frontmatter: bool,
) -> Result<PromptData> {
    let stem = file.file_stem().unwrap().to_str().unwrap().to_string();
    let rel_path = file.strip_prefix(folder).unwrap().display().to_string();
    let default_description = format!("Prompt from {}", rel_path);

    if skip_frontmatter {
        return Ok(PromptData {
            name: stem.clone(),
            title: stem,
            description: default_description,
            arguments: vec![],
            content: content.trim().to_string(),
        });
    }

    // Parse frontmatter manually
    let (frontmatter, body) = if content.starts_with("---\n") {
        if let Some(end_idx) = content[4..].find("\n---\n") {
            let fm = &content[4..end_idx + 4];
            let body = &content[end_idx + 8..];
            (Some(fm), body)
        } else {
            (None, content)
        }
    } else {
        (None, content)
    };

    let mut name = stem.clone();
    let mut title = stem.clone();
    let mut description = default_description.clone();
    let mut arguments = Vec::new();

    if let Some(fm) = frontmatter {
        if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(fm) {
            if let Some(mapping) = yaml.as_mapping() {
                // Extract name field
                if let Some(n) = mapping.get("name") {
                    if let Some(s) = n.as_str() {
                        name = s.to_string();
                    } else {
                        eprintln!(
                            "Warning: 'name' field in {} is not a string, converting to string",
                            file.display()
                        );
                        name = n.as_str().unwrap_or(&format!("{:?}", n)).to_string();
                    }
                }

                // Extract title field
                if let Some(t) = mapping.get("title") {
                    if let Some(s) = t.as_str() {
                        title = s.to_string();
                    } else {
                        eprintln!(
                            "Warning: 'title' field in {} is not a string, converting to string",
                            file.display()
                        );
                        title = t.as_str().unwrap_or(&format!("{:?}", t)).to_string();
                    }
                }

                // Extract description field
                if let Some(d) = mapping.get("description") {
                    if let Some(s) = d.as_str() {
                        description = s.to_string();
                    } else {
                        eprintln!("Warning: 'description' field in {} is not a string, converting to string", file.display());
                        description = d.as_str().unwrap_or(&format!("{:?}", d)).to_string();
                    }
                }

                // Extract arguments
                if let Some(args_value) = mapping.get("arguments") {
                    if let Some(args) = args_value.as_sequence() {
                        for item in args {
                            if let Some(arg_map) = item.as_mapping() {
                                // Parse argument name (required)
                                let arg_name = if let Some(n) = arg_map.get("name") {
                                    if let Some(s) = n.as_str() {
                                        if s.is_empty() {
                                            eprintln!("Warning: argument 'name' field is empty in {}, skipping argument", file.display());
                                            continue;
                                        }
                                        // Validate variable name
                                        if !crate::formatter::validate_variable_name(s) {
                                            return Err(anyhow::anyhow!(
                                                "Argument name '{}' contains invalid characters",
                                                s
                                            ));
                                        }
                                        s.to_string()
                                    } else {
                                        eprintln!("Warning: argument 'name' field in {} is not a string, converting to string", file.display());
                                        let converted = format!("{:?}", n);
                                        if converted.is_empty() {
                                            continue;
                                        }
                                        converted
                                    }
                                } else {
                                    eprintln!("Warning: argument 'name' field is missing in {}, skipping argument", file.display());
                                    continue;
                                };

                                // Parse description (optional)
                                let arg_description = if let Some(d) = arg_map.get("description") {
                                    if let Some(s) = d.as_str() {
                                        s.to_string()
                                    } else {
                                        eprintln!("Warning: argument 'description' field in {} is not a string, converting to string", file.display());
                                        format!("{:?}", d)
                                    }
                                } else {
                                    String::new()
                                };

                                // Parse default (optional)
                                let arg_default = if let Some(def) = arg_map.get("default") {
                                    if let Some(s) = def.as_str() {
                                        Some(s.to_string())
                                    } else {
                                        eprintln!("Warning: argument 'default' field in {} is not a string, converting to string", file.display());
                                        Some(format!("{:?}", def))
                                    }
                                } else {
                                    None
                                };

                                arguments.push(Argument {
                                    name: arg_name,
                                    description: arg_description,
                                    default: arg_default,
                                });
                            } else {
                                eprintln!(
                                    "Warning: argument item in {} is not a dict, skipping",
                                    file.display()
                                );
                            }
                        }
                    } else if !args_value.is_null() {
                        eprintln!(
                            "Warning: 'arguments' field in {} is not a list, ignoring",
                            file.display()
                        );
                    }
                }
            }
        }
    }

    Ok(PromptData {
        name,
        title,
        description,
        arguments,
        content: body.trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_git_url_github_https() {
        let (owner, name) = parse_git_url("https://github.com/user/repo.git").unwrap();
        assert_eq!(owner, "user");
        assert_eq!(name, "repo");
    }

    #[test]
    fn test_parse_git_url_github_ssh() {
        let (owner, name) = parse_git_url("git@github.com:user/repo.git").unwrap();
        assert_eq!(owner, "user");
        assert_eq!(name, "repo");
    }

    #[test]
    fn test_parse_git_url_gitlab_https() {
        let (owner, name) = parse_git_url("https://gitlab.com/user/repo.git").unwrap();
        assert_eq!(owner, "user");
        assert_eq!(name, "repo");
    }

    #[test]
    fn test_parse_git_url_gitlab_ssh() {
        let (owner, name) = parse_git_url("git@gitlab.com:user/repo.git").unwrap();
        assert_eq!(owner, "user");
        assert_eq!(name, "repo");
    }

    #[test]
    fn test_parse_git_url_with_username() {
        let (owner, name) = parse_git_url("https://username@github.com/owner/repo.git").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(name, "repo");
    }

    #[test]
    fn test_parse_git_url_with_credentials() {
        let (owner, name) =
            parse_git_url("https://username:token@github.com/owner/repo.git").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(name, "repo");
    }

    #[test]
    fn test_parse_git_url_invalid() {
        let result = parse_git_url("invalid-url");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_cache_path() {
        let path = get_cache_path("https://github.com/user/repo.git", "/cache").unwrap();
        assert_eq!(path, PathBuf::from("/cache/git/user/repo"));
    }

    #[test]
    fn test_get_folder_path_local() {
        let result = get_folder_path(Some("/local/path"), None, "/cache", false).unwrap();
        assert_eq!(result, PathBuf::from("/local/path"));
    }

    #[test]
    fn test_get_folder_path_no_config() {
        let result = get_folder_path(None, None, "/cache", false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Either folder or git-url must be provided"));
    }
}
