//! Reading files from the tree of a commit, without checking it out.

use std::error::Error;
use std::path::{Path, PathBuf};

use git2::{ObjectType, TreeWalkMode, TreeWalkResult};

use super::GitRepo;

impl GitRepo {
    /// The files of the tree at `refspec` that `wanted` picks, with their
    /// content. Directories `skipped` rejects are not entered, and a file
    /// that is not text is left out.
    pub fn files_at(
        &self,
        refspec: &str,
        skipped: impl Fn(&Path) -> bool,
        wanted: impl Fn(&Path) -> bool,
    ) -> Result<Vec<(PathBuf, String)>, Box<dyn Error>> {
        let tree = self.ref_tree(refspec)?;
        let mut files = Vec::new();
        tree.walk(TreeWalkMode::PreOrder, |dir, entry| {
            let path = Path::new(dir).join(entry.name().unwrap_or_default());
            match entry.kind() {
                Some(ObjectType::Tree) if skipped(&path) => return TreeWalkResult::Skip,
                Some(ObjectType::Blob) if wanted(&path) => {
                    let text = self
                        .repo
                        .find_blob(entry.id())
                        .ok()
                        .and_then(|blob| String::from_utf8(blob.content().to_vec()).ok());
                    files.extend(text.map(|text| (path, text)));
                }
                _ => {}
            }
            TreeWalkResult::Ok
        })?;
        Ok(files)
    }

    /// Whether `path` is a file in the tree at `refspec`.
    pub fn has_file_at(&self, refspec: &str, path: &Path) -> bool {
        self.ref_tree(refspec)
            .and_then(|tree| Ok(tree.get_path(path)?))
            .is_ok_and(|entry| entry.kind() == Some(ObjectType::Blob))
    }
}
