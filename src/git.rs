use anyhow::Result;
use std::process::Command;

#[allow(dead_code)]
pub fn clone(github_repository: &str, dest: &std::path::Path) -> Result<()> {
    let repo_url = format!("git@github.com:{}.git", github_repository);
    let status = Command::new("git")
        .arg("clone")
        .arg(repo_url)
        .arg(dest)
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to execute git clone: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to clone repository"))
    }
}
