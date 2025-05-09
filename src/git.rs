use anyhow::Result;
use git2::{BranchType, Repository};

pub fn init_symlink() -> Result<()> {
    let mut path = dirs::home_dir().expect("No valid home dir found");
    path.push(".cargo/bin/cargo-preflight");
    std::os::unix::fs::symlink(&path, "./.git/hooks/pre-commit")?;
    std::os::unix::fs::symlink(&path, "./.git/hooks/pre-push")?;
    Ok(())
}

pub fn delete_symlink() -> Result<()> {
    std::fs::remove_file("./.git/hooks/pre-commit")?;
    std::fs::remove_file("./.git/hooks/pre-push")?;
    Ok(())
}

pub fn get_current_branch_name() -> Option<String> {
    let repo = Repository::open(".").ok()?;
    let head = repo.head().ok()?;
    head.shorthand().map(String::from)
}

pub fn get_branches() -> Result<Vec<String>, git2::Error> {
    let repo = Repository::open(".")?;

    // Collect all branches into a vector
    let branches = repo
        .branches(Some(BranchType::Local))?
        .filter_map(|branch_result| match branch_result {
            Ok((branch, _)) => branch.name().ok().flatten().map(String::from),
            Err(_) => None, // Ignore branches that fail to load
        })
        .collect();

    Ok(branches)
}
