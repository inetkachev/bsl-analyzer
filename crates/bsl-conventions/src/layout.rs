//! Which layout a source root uses.
//!
//! A configurator dump and an EDT project describe the same configuration with
//! different files in different places, and the difference is decided once per
//! root: by the descriptor that only an EDT root carries. Everything below the
//! root (which sibling names an object, where a module body lives) is written
//! against this answer.

use std::path::{Path, PathBuf};

use crate::{
    conventional_of, find_child_ci_in, ConventionalName, DirTree, EntryKind, RealFs, SegmentMatch,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceFormat {
    /// A configurator dump: `Configuration.xml` at the root, `<Name>.xml` beside
    /// each object directory, service level `Ext/`.
    Designer,
    /// An EDT project's `src`: `Configuration/Configuration.mdo`, `<Name>.mdo`
    /// inside each object directory, no service level.
    Edt,
}

impl SourceFormat {
    /// The layout of `root`. Only the EDT descriptor decides: a root without it
    /// reads as a configurator dump, which is also what a bare directory of
    /// objects (no `Configuration.xml`) has always been read as.
    pub fn detect(root: &Path, tree: &dyn DirTree) -> Self {
        if edt_configuration_mdo_in(root, tree).is_some() {
            SourceFormat::Edt
        } else {
            SourceFormat::Designer
        }
    }

    pub fn of_directory(root: &Path) -> Self {
        Self::detect(root, &RealFs)
    }

    pub fn is_edt(self) -> bool {
        self == SourceFormat::Edt
    }
}

/// `<root>/Configuration/Configuration.mdo` as a FILE, in whatever ASCII case
/// the tree spells the two conventional segments.
pub fn edt_configuration_mdo_in(root: &Path, tree: &dyn DirTree) -> Option<PathBuf> {
    let dir = find_child_ci_in(tree, root, ConventionalName::ConfigurationDir.canonical())?;
    let mdo = find_child_ci_in(tree, &dir, ConventionalName::ConfigurationMdo.canonical())?;
    (tree.kind_of(&mdo) == Some(EntryKind::File)).then_some(mdo)
}

/// The spelling an EDT project gives to a module path CONSTRUCTED for a
/// configurator dump: the service level goes away (`…/X/Ext/ObjectModule.bsl`
/// is `…/X/ObjectModule.bsl`, `…/Forms/F/Ext/Form/Module.bsl` is
/// `…/Forms/F/Module.bsl`) and a root module moves from `Ext/` to
/// `Configuration/`. The match modes that came with the tail are trimmed the
/// same way, so a caller that built its candidate for one layout resolves it
/// in the other without knowing which one the root uses. `None` when the
/// components carry no service level.
pub fn edt_twin(
    components: &[String],
    tail_modes: &[SegmentMatch],
) -> Option<(Vec<String>, Vec<SegmentMatch>)> {
    let n = components.len();
    let is = |i: usize, name: ConventionalName| conventional_of(&components[i]) == Some(name);
    let mut twin = components.to_vec();
    let mut modes = tail_modes.to_vec();
    // The modes describe the last `tail_modes.len()` components; a component
    // dropped inside that region takes its mode with it.
    let head = n.saturating_sub(tail_modes.len());
    let drop_at = |i: usize, twin: &mut Vec<String>, modes: &mut Vec<SegmentMatch>| {
        if i >= head {
            modes.remove(i - head);
        }
        twin.remove(i);
    };

    if n >= 5
        && is(n - 3, ConventionalName::Ext)
        && is(n - 2, ConventionalName::Form)
        && is(n - 1, ConventionalName::Module)
    {
        drop_at(n - 2, &mut twin, &mut modes);
        drop_at(n - 3, &mut twin, &mut modes);
        return Some((twin, modes));
    }
    if n >= 2 && is(n - 2, ConventionalName::Ext) {
        let root_module = matches!(
            conventional_of(&components[n - 1]),
            Some(
                ConventionalName::SessionModule
                    | ConventionalName::ExternalConnectionModule
                    | ConventionalName::ManagedApplicationModule
                    | ConventionalName::OrdinaryApplicationModule
                    | ConventionalName::ApplicationModule
            )
        );
        if root_module {
            twin[n - 2] = ConventionalName::ConfigurationDir.canonical().to_string();
        } else {
            drop_at(n - 2, &mut twin, &mut modes);
        }
        return Some((twin, modes));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PathSetTree;

    fn files(paths: &[&str]) -> PathSetTree {
        PathSetTree::from_files(paths.iter().map(PathBuf::from))
    }

    #[test]
    fn the_edt_marker_wins_and_the_designer_marker_or_nothing_means_designer() {
        let root = Path::new("/ws/src");
        let edt =
            files(&["/ws/src/Configuration/Configuration.mdo", "/ws/src/CommonModules/A/A.mdo"]);
        assert_eq!(SourceFormat::detect(root, &edt), SourceFormat::Edt);
        let designer = files(&["/ws/src/Configuration.xml"]);
        assert_eq!(SourceFormat::detect(root, &designer), SourceFormat::Designer);
        let bare = files(&["/ws/src/CommonModules/A.xml"]);
        assert_eq!(SourceFormat::detect(root, &bare), SourceFormat::Designer);
    }

    #[test]
    fn the_marker_is_found_in_any_ascii_case_of_its_conventional_segments() {
        let root = Path::new("/ws/src");
        let tree = files(&["/ws/src/CONFIGURATION/configuration.MDO"]);
        assert_eq!(SourceFormat::detect(root, &tree), SourceFormat::Edt);
        assert!(edt_configuration_mdo_in(root, &tree).unwrap().ends_with("configuration.MDO"));
    }

    #[test]
    fn a_configuration_directory_alone_is_not_a_marker() {
        // A configurator dump with a catalog named `Configuration` has this
        // directory too; only the descriptor inside it names the EDT layout.
        let tree = files(&["/ws/src/Configuration/SessionModule.bsl"]);
        assert_eq!(SourceFormat::detect(Path::new("/ws/src"), &tree), SourceFormat::Designer);
    }

    fn parts(path: &str) -> Vec<String> {
        path.split('/').map(str::to_string).collect()
    }

    #[test]
    fn a_designer_candidate_has_an_edt_twin() {
        use SegmentMatch as M;
        let (twin, modes) = edt_twin(
            &parts("w/cf/Catalogs/X/Ext/ManagerModule.bsl"),
            &[M::Ci, M::Exact, M::Ci, M::Ci],
        )
        .unwrap();
        assert_eq!(twin, parts("w/cf/Catalogs/X/ManagerModule.bsl"));
        assert_eq!(modes, vec![M::Ci, M::Exact, M::Ci]);

        let (twin, modes) = edt_twin(
            &parts("w/Catalogs/X/Forms/F/Ext/Form/Module.bsl"),
            &[M::Ci, M::Exact, M::Ci, M::Exact, M::Ci, M::Ci, M::Ci],
        )
        .unwrap();
        assert_eq!(twin, parts("w/Catalogs/X/Forms/F/Module.bsl"));
        assert_eq!(modes, vec![M::Ci, M::Exact, M::Ci, M::Exact, M::Ci]);

        let (twin, modes) =
            edt_twin(&parts("w/cf/Ext/SessionModule.bsl"), &[M::Ci, M::Ci]).unwrap();
        assert_eq!(twin, parts("w/cf/Configuration/SessionModule.bsl"));
        assert_eq!(modes, vec![M::Ci, M::Ci]);

        // A dropped component outside the mode tail leaves the modes alone.
        let (twin, modes) = edt_twin(&parts("w/CommonModules/M/Ext/Module.bsl"), &[M::Ci]).unwrap();
        assert_eq!(twin, parts("w/CommonModules/M/Module.bsl"));
        assert_eq!(modes, vec![M::Ci]);

        assert_eq!(edt_twin(&parts("w/CommonModules/M/Module.bsl"), &[M::Ci]), None);
        assert_eq!(edt_twin(&parts("Module.bsl"), &[M::Ci]), None);
    }

    #[test]
    fn of_directory_reads_the_real_filesystem() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(SourceFormat::of_directory(dir.path()), SourceFormat::Designer);
        std::fs::create_dir_all(dir.path().join("Configuration")).unwrap();
        std::fs::write(dir.path().join("Configuration/Configuration.mdo"), "<x/>").unwrap();
        assert_eq!(SourceFormat::of_directory(dir.path()), SourceFormat::Edt);
    }
}
