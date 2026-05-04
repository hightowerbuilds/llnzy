use std::collections::HashSet;
use std::path::Path;

use crate::path_utils::{comparable_path, path_contains, same_path};

use super::model::{SidebarMovePlan, SidebarMovePlanItem, SidebarMoveRequest};

pub fn plan_sidebar_move(request: &SidebarMoveRequest) -> Result<SidebarMovePlan, String> {
    if request.sources.is_empty() {
        return Err("No files or folders selected to move".to_string());
    }
    if !request.destination_folder.is_dir() {
        return Err(format!(
            "Move target is not a folder: {}",
            request.destination_folder.display()
        ));
    }

    let destination_key = comparable_path(&request.destination_folder);
    let mut seen_sources = HashSet::new();
    let mut seen_destinations = HashSet::new();
    let mut items = Vec::with_capacity(request.sources.len());

    for source in &request.sources {
        if !source.exists() {
            return Err(format!(
                "Move source no longer exists: {}",
                source.display()
            ));
        }
        let source_key = comparable_path(source);
        if !seen_sources.insert(source_key.clone()) {
            return Err(format!("Duplicate move source: {}", source.display()));
        }

        let is_dir = source.is_dir();
        if !is_dir && !source.is_file() {
            return Err(format!(
                "Only files and folders can be moved: {}",
                source.display()
            ));
        }
        if same_path(source, &request.destination_folder) {
            return Err("Cannot move a folder into itself".to_string());
        }
        if is_dir && path_contains(source, &request.destination_folder) {
            return Err(format!(
                "Cannot move {} into one of its own folders",
                display_name(source)
            ));
        }
        if source.parent().map(comparable_path) == Some(destination_key.clone()) {
            return Err(format!(
                "{} is already in {}",
                display_name(source),
                request.destination_folder.display()
            ));
        }

        let Some(file_name) = source.file_name() else {
            return Err(format!("Cannot determine name for {}", source.display()));
        };
        let destination = request.destination_folder.join(file_name);
        if destination.exists() && !same_path(source, &destination) {
            return Err(format!(
                "{} already exists in {}",
                file_name.to_string_lossy(),
                request.destination_folder.display()
            ));
        }
        if !seen_destinations.insert(comparable_path(&destination)) {
            return Err(format!(
                "Multiple moved items would land at {}",
                destination.display()
            ));
        }

        items.push(SidebarMovePlanItem {
            source: source.clone(),
            destination,
            is_dir,
        });
    }

    Ok(SidebarMovePlan {
        destination_folder: request.destination_folder.clone(),
        items,
    })
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Item")
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;
    use crate::sidebar_move::MoveOrigin;

    #[test]
    fn plans_file_move_to_folder() {
        let root = temp_root("file-move");
        let source_dir = root.join("src");
        let dest_dir = root.join("archive");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&dest_dir).unwrap();
        let source = source_dir.join("note.md");
        fs::write(&source, "note").unwrap();

        let request =
            SidebarMoveRequest::new(vec![source.clone()], dest_dir.clone(), MoveOrigin::DragDrop);
        let plan = plan_sidebar_move(&request).unwrap();

        assert_eq!(plan.items[0].source, source);
        assert_eq!(plan.items[0].destination, dest_dir.join("note.md"));
        assert!(!plan.items[0].is_dir);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn plans_folder_move_to_folder() {
        let root = temp_root("folder-move");
        let source = root.join("docs");
        let dest_dir = root.join("archive");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&dest_dir).unwrap();

        let request =
            SidebarMoveRequest::new(vec![source.clone()], dest_dir.clone(), MoveOrigin::DragDrop);
        let plan = plan_sidebar_move(&request).unwrap();

        assert_eq!(plan.items[0].destination, dest_dir.join("docs"));
        assert!(plan.items[0].is_dir);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_moving_folder_into_descendant() {
        let root = temp_root("folder-descendant");
        let source = root.join("docs");
        let child = source.join("nested");
        fs::create_dir_all(&child).unwrap();

        let request = SidebarMoveRequest::new(vec![source], child, MoveOrigin::DragDrop);
        let error = plan_sidebar_move(&request).unwrap_err();

        assert!(error.contains("own folders"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_existing_destination() {
        let root = temp_root("collision");
        let source_dir = root.join("src");
        let dest_dir = root.join("archive");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&dest_dir).unwrap();
        let source = source_dir.join("note.md");
        fs::write(&source, "note").unwrap();
        fs::write(dest_dir.join("note.md"), "existing").unwrap();

        let request = SidebarMoveRequest::new(vec![source], dest_dir, MoveOrigin::ContextMenu);
        let error = plan_sidebar_move(&request).unwrap_err();

        assert!(error.contains("already exists"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_same_parent_folder() {
        let root = temp_root("same-parent");
        let source = root.join("note.md");
        fs::create_dir_all(&root).unwrap();
        fs::write(&source, "note").unwrap();

        let request = SidebarMoveRequest::new(vec![source], root.clone(), MoveOrigin::DragDrop);
        let error = plan_sidebar_move(&request).unwrap_err();

        assert!(error.contains("already in"));

        let _ = fs::remove_dir_all(root);
    }

    fn temp_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("llnzy-sidebar-move-{}-{label}", std::process::id()))
    }
}
