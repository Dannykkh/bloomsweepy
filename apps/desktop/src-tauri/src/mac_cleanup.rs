//! Fixed developer-data roots for the existing bounded cleanup engine.
//! Only the fixed path components below the supplied home are inspected for
//! links. Contents are never enumerated here and no Trash operation is approved.
use bloomsweepy_core::{CleanupCandidateKind, CleanupRootSpec};
use std::fs;
use std::path::Path;
use std::time::Duration;

pub(crate) fn developer_cleanup_roots(home: &Path) -> Vec<CleanupRootSpec> {
    let xcode = home.join("Library/Developer/Xcode");
    vec![
        CleanupRootSpec::new(
            xcode.join("DerivedData"),
            "Xcode 빌드 캐시",
            CleanupCandidateKind::DeveloperCache,
            Duration::from_secs(7 * 86_400),
        ),
        CleanupRootSpec::new(
            xcode.join("iOS DeviceSupport"),
            "iOS 기기 지원 데이터",
            CleanupCandidateKind::DeveloperCache,
            Duration::from_secs(30 * 86_400),
        ),
    ]
    .into_iter()
    .filter(|root| !developer_path_contains_link(home, &root.path))
    .collect()
}

fn developer_path_contains_link(home: &Path, root: &Path) -> bool {
    let Ok(relative) = root.strip_prefix(home) else {
        return true;
    };
    let mut path = home.to_path_buf();
    // Home itself and its ancestors may legitimately use macOS aliases such as
    // /var -> /private/var. Check only Library/Developer/Xcode/<fixed root>.
    // Missing or unreadable paths retain the cleanup scanner's existing error
    // handling; a clearly identified link is never offered as a trusted root.
    for component in relative.components() {
        path.push(component);
        if fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn developer_cleanup_catalog_has_only_two_fixed_age_gated_roots() {
        // Missing roots remain catalog entries without enumerating or creating
        // any developer data, so the scanner can report their usual state.
        let fixture = tempfile::tempdir().unwrap();
        let home = fixture.path().join("nonexistent-home");
        let roots = developer_cleanup_roots(&home);
        assert!(!home.exists());
        assert_eq!(roots.len(), 2);
        assert_eq!(
            roots[0].path,
            home.join("Library/Developer/Xcode/DerivedData")
        );
        assert_eq!(roots[0].minimum_age, Duration::from_secs(7 * 86_400));
        assert_eq!(
            roots[1].path,
            home.join("Library/Developer/Xcode/iOS DeviceSupport")
        );
        assert_eq!(roots[1].minimum_age, Duration::from_secs(30 * 86_400));
        assert!(roots.iter().all(|root| {
            root.kind == CleanupCandidateKind::DeveloperCache && root.protected_names.is_empty()
        }));
        assert_ne!(roots[0].label, roots[1].label);
        for root in roots {
            let value = root.path.to_string_lossy();
            for excluded in [
                "Application Support",
                "Documents",
                "Archives",
                "CoreSimulator",
                "MobileAsset",
            ] {
                assert!(
                    !value.contains(excluded),
                    "unexpected cleanup root: {value}"
                );
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn developer_cleanup_rejects_links_in_each_parent_component() {
        use std::os::unix::fs::symlink;

        for component in ["Library", "Library/Developer", "Library/Developer/Xcode"] {
            let fixture = tempfile::tempdir().unwrap();
            let home = fixture.path().join("home");
            let outside = fixture.path().join("outside");
            fs::create_dir_all(&outside).unwrap();
            fs::write(outside.join("preserve.txt"), b"not developer data").unwrap();
            let linked = home.join(component);
            fs::create_dir_all(linked.parent().unwrap()).unwrap();
            symlink(&outside, &linked).unwrap();

            assert!(
                developer_cleanup_roots(&home).is_empty(),
                "linked parent must exclude both roots: {component}"
            );
            assert_eq!(
                fs::read(outside.join("preserve.txt")).unwrap(),
                b"not developer data"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn developer_cleanup_rejects_a_linked_root_without_excluding_its_sibling() {
        use std::os::unix::fs::symlink;

        let fixture = tempfile::tempdir().unwrap();
        let home = fixture.path().join("home");
        let xcode = home.join("Library/Developer/Xcode");
        fs::create_dir_all(&xcode).unwrap();
        // symlink_metadata also identifies a dangling link, even though the
        // target cannot be canonicalized or enumerated.
        symlink(
            fixture.path().join("missing-target"),
            xcode.join("DerivedData"),
        )
        .unwrap();

        let roots = developer_cleanup_roots(&home);
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].path, xcode.join("iOS DeviceSupport"));
    }

    #[cfg(unix)]
    #[test]
    fn developer_cleanup_does_not_reject_links_at_or_above_home() {
        use std::os::unix::fs::symlink;

        let fixture = tempfile::tempdir().unwrap();
        let real_home = fixture.path().join("actual-home");
        fs::create_dir_all(real_home.join("Library/Developer/Xcode")).unwrap();
        let home_alias = fixture.path().join("home-alias");
        symlink(&real_home, &home_alias).unwrap();

        let roots = developer_cleanup_roots(&home_alias);
        assert_eq!(roots.len(), 2);
        assert!(roots.iter().all(|root| root.path.starts_with(&home_alias)));
    }
}
