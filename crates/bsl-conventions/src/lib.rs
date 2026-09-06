//! The single policy point for the configurator dump's conventional names.
//!
//! A configurator dump spells its structural vocabulary — extensions, service
//! directories, module file names, service XML names — in a fixed canonical
//! form, but tools that produced a tree may have written any ASCII case. Every
//! comparison of a conventional name in the workspace therefore goes through
//! this crate: ASCII-case-insensitive, with the canonical spelling as the one
//! source of truth for construction. Object NAMES (the segment an author chose)
//! are never compared case-insensitively — that distinction is positional, so
//! the caller decides which segments are conventional, never this crate from
//! the spelling alone (an object may legally be named `Ext`).
//!
//! Collection directory names (`Catalogs`, `Отчёты`, …) are NOT here: they are
//! bilingual and their equivalence — including `ё` variants and Unicode
//! composition — is owned by `bsl_metadata`'s module-path spec.

mod layout;
mod probe;
mod tree;

pub use layout::{edt_configuration_mdo_in, edt_twin, SourceFormat};
pub use probe::{
    find_child_ci, find_child_ci_in, find_child_stem_exact, find_child_stem_exact_in,
    resolve_chain_ci, resolve_chain_ci_in,
};
pub use tree::{DirTree, EntryKind, PathSetTree, RealFs, TreeEntry};

/// Every conventional name the dump layout uses and the workspace compares.
///
/// The variants are the dictionary: the completeness gate and the table tests
/// both iterate [`ConventionalName::ALL`], so a name missing here is missing
/// everywhere visibly, not silently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConventionalName {
    // Module file names.
    Module,
    ObjectModule,
    ManagerModule,
    FormModule,
    CommandModule,
    RecordSetModule,
    ValueManagerModule,
    SessionModule,
    ExternalConnectionModule,
    ManagedApplicationModule,
    OrdinaryApplicationModule,
    ApplicationModule,
    /// The protected-module container next to a missing `Module.bsl`.
    ModuleBin,
    // Service directories.
    Ext,
    Form,
    Forms,
    Commands,
    // Service XML files.
    ConfigurationXml,
    PredefinedXml,
    RightsXml,
    FormXml,
    // EDT project layout.
    /// The `Configuration/` directory at an EDT source root: holds the
    /// configuration descriptor and the root modules.
    ConfigurationDir,
    ConfigurationMdo,
    FormForm,
    RightsRights,
}

impl ConventionalName {
    pub const ALL: &'static [ConventionalName] = &[
        ConventionalName::Module,
        ConventionalName::ObjectModule,
        ConventionalName::ManagerModule,
        ConventionalName::FormModule,
        ConventionalName::CommandModule,
        ConventionalName::RecordSetModule,
        ConventionalName::ValueManagerModule,
        ConventionalName::SessionModule,
        ConventionalName::ExternalConnectionModule,
        ConventionalName::ManagedApplicationModule,
        ConventionalName::OrdinaryApplicationModule,
        ConventionalName::ApplicationModule,
        ConventionalName::ModuleBin,
        ConventionalName::Ext,
        ConventionalName::Form,
        ConventionalName::Forms,
        ConventionalName::Commands,
        ConventionalName::ConfigurationXml,
        ConventionalName::PredefinedXml,
        ConventionalName::RightsXml,
        ConventionalName::FormXml,
        ConventionalName::ConfigurationDir,
        ConventionalName::ConfigurationMdo,
        ConventionalName::FormForm,
        ConventionalName::RightsRights,
    ];

    /// The canonical spelling the configurator writes. Constructing a path or
    /// URI uses this; comparing an on-disk name never does directly — that is
    /// what [`conventional_of`] is for.
    pub const fn canonical(self) -> &'static str {
        match self {
            ConventionalName::Module => "Module.bsl",
            ConventionalName::ObjectModule => "ObjectModule.bsl",
            ConventionalName::ManagerModule => "ManagerModule.bsl",
            ConventionalName::FormModule => "FormModule.bsl",
            ConventionalName::CommandModule => "CommandModule.bsl",
            ConventionalName::RecordSetModule => "RecordSetModule.bsl",
            ConventionalName::ValueManagerModule => "ValueManagerModule.bsl",
            ConventionalName::SessionModule => "SessionModule.bsl",
            ConventionalName::ExternalConnectionModule => "ExternalConnectionModule.bsl",
            ConventionalName::ManagedApplicationModule => "ManagedApplicationModule.bsl",
            ConventionalName::OrdinaryApplicationModule => "OrdinaryApplicationModule.bsl",
            ConventionalName::ApplicationModule => "ApplicationModule.bsl",
            ConventionalName::ModuleBin => "Module.bin",
            ConventionalName::Ext => "Ext",
            ConventionalName::Form => "Form",
            ConventionalName::Forms => "Forms",
            ConventionalName::Commands => "Commands",
            ConventionalName::ConfigurationXml => "Configuration.xml",
            ConventionalName::PredefinedXml => "Predefined.xml",
            ConventionalName::RightsXml => "Rights.xml",
            ConventionalName::FormXml => "Form.xml",
            ConventionalName::ConfigurationDir => "Configuration",
            ConventionalName::ConfigurationMdo => "Configuration.mdo",
            ConventionalName::FormForm => "Form.form",
            ConventionalName::RightsRights => "Rights.rights",
        }
    }

    /// The canonical stem, for the callers that compare after `file_stem()`.
    pub fn canonical_stem(self) -> &'static str {
        let canonical = self.canonical();
        match canonical.rsplit_once('.') {
            Some((stem, _)) => stem,
            None => canonical,
        }
    }
}

/// Recognize a conventional name in any ASCII case. `None` means the name is
/// not conventional — which for a path segment usually means it is an object
/// name and must be treated exactly.
///
/// `Form`/`Form.xml` and `Module.bsl`-vs-stem ambiguity is on the caller: pass
/// a full file name to get file-name variants, a directory name to get
/// directory variants; this function only answers "which dictionary entry does
/// this spelling collapse into", preferring the first match in
/// [`ConventionalName::ALL`] order (the canonical spellings are distinct, so
/// at most one matches).
pub fn conventional_of(name: &str) -> Option<ConventionalName> {
    ConventionalName::ALL.iter().copied().find(|c| c.canonical().eq_ignore_ascii_case(name))
}

/// The source extension of a `.bsl` module body.
pub const BSL_EXTENSION: &str = "bsl";
/// The extension of metadata descriptors.
pub const XML_EXTENSION: &str = "xml";
/// The extension of protected (compiled) module containers.
pub const BIN_EXTENSION: &str = "bin";
/// The extension of EDT metadata descriptors.
pub const MDO_EXTENSION: &str = "mdo";

/// Case-insensitive extension check on a path, the one way the workspace asks
/// "is this a `.bsl` / `.xml` file".
pub fn has_extension(path: &std::path::Path, extension: &str) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case(extension))
}

/// [`has_extension`] for extensions already carried as strings (event keys,
/// URIs), matching on the suffix after the last dot.
pub fn str_has_extension(name: &str, extension: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(_, e)| e.eq_ignore_ascii_case(extension))
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;

    #[test]
    fn every_spelling_of_a_conventional_name_collapses_into_its_entry() {
        for &name in ConventionalName::ALL {
            let canonical = name.canonical();
            assert_eq!(conventional_of(canonical), Some(name), "{canonical}");
            assert_eq!(conventional_of(&canonical.to_ascii_uppercase()), Some(name));
            assert_eq!(conventional_of(&canonical.to_ascii_lowercase()), Some(name));
        }
    }

    #[test]
    fn an_object_name_is_not_conventional() {
        for name in ["Товары", "Alpha", "ExtModule", "ModuleX", "Configurations"] {
            assert_eq!(conventional_of(name), None, "{name}");
        }
    }

    /// The dictionary is data other checks are built from, so its composition
    /// is pinned independently: an accidental removal or rename shows up as a
    /// snapshot diff here even though the gate and the table tests would have
    /// silently followed the change.
    #[test]
    fn the_dictionary_composition_is_pinned() {
        let listing =
            ConventionalName::ALL.iter().map(|c| c.canonical()).collect::<Vec<_>>().join("\n");
        expect![[r#"
            Module.bsl
            ObjectModule.bsl
            ManagerModule.bsl
            FormModule.bsl
            CommandModule.bsl
            RecordSetModule.bsl
            ValueManagerModule.bsl
            SessionModule.bsl
            ExternalConnectionModule.bsl
            ManagedApplicationModule.bsl
            OrdinaryApplicationModule.bsl
            ApplicationModule.bsl
            Module.bin
            Ext
            Form
            Forms
            Commands
            Configuration.xml
            Predefined.xml
            Rights.xml
            Form.xml
            Configuration
            Configuration.mdo
            Form.form
            Rights.rights"#]]
        .assert_eq(&listing);
    }

    #[test]
    fn extension_checks_ignore_ascii_case_only() {
        use std::path::Path;
        assert!(has_extension(Path::new("a/Module.BSL"), BSL_EXTENSION));
        assert!(has_extension(Path::new("a/Meta.XML"), XML_EXTENSION));
        assert!(!has_extension(Path::new("a/Module.bs"), BSL_EXTENSION));
        assert!(!has_extension(Path::new("a/Module"), BSL_EXTENSION));
        assert!(str_has_extension("Meta.XML", XML_EXTENSION));
        assert!(!str_has_extension("Meta", XML_EXTENSION));
    }

    #[test]
    fn edt_names_are_in_the_dictionary() {
        assert_eq!(conventional_of("Configuration"), Some(ConventionalName::ConfigurationDir));
        assert_eq!(conventional_of("configuration.MDO"), Some(ConventionalName::ConfigurationMdo));
        assert_eq!(conventional_of("form.FORM"), Some(ConventionalName::FormForm));
        assert_eq!(conventional_of("Rights.rights"), Some(ConventionalName::RightsRights));
        assert!(str_has_extension("Справочник1.mdo", MDO_EXTENSION));
    }

    #[test]
    fn a_root_module_is_recognised_under_ext_and_under_configuration() {
        for path in [
            "src/cf/Ext/SessionModule.bsl",
            r"C:\p\src\Ext\SessionModule.bsl",
            "src/Configuration/SessionModule.bsl",
            r"C:\p\src\Configuration\sessionmodule.bsl",
        ] {
            assert!(path_is_root_module(path, ConventionalName::SessionModule), "{path}");
            assert!(path_ends_with_ext_child(path, ConventionalName::SessionModule), "{path}");
        }
        assert!(!path_is_root_module(
            "Catalogs/Configuration/ObjectModule.bsl",
            ConventionalName::SessionModule
        ));
        assert!(!path_is_root_module(
            "src/Other/SessionModule.bsl",
            ConventionalName::SessionModule
        ));
        assert!(!path_is_root_module("SessionModule.bsl", ConventionalName::SessionModule));
    }

    #[test]
    fn a_stem_maps_to_its_file_name_entry() {
        assert_eq!(ConventionalName::ObjectModule.canonical_stem(), "ObjectModule");
        assert_eq!(ConventionalName::Ext.canonical_stem(), "Ext");
    }
}

/// How one path component of a CONSTRUCTED candidate matches a real component.
/// The caller decides per position — deriving the mode from the spelling would
/// swallow an object legally named `Ext`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentMatch {
    /// An object/form/command NAME: its case is significant.
    Exact,
    /// A wholly conventional segment (`Ext`, `Forms`, `Module.bsl`, a
    /// collection directory): ASCII-case-insensitive.
    Ci,
    /// A `{name}.{ext}` file built from an object name: the stem is exact,
    /// only the extension folds.
    StemExactExtCi,
}

impl SegmentMatch {
    /// Whether a real path component satisfies this mode against the
    /// constructed candidate component.
    pub fn matches(self, real: &str, candidate: &str) -> bool {
        match self {
            SegmentMatch::Exact => real == candidate,
            SegmentMatch::Ci => real.eq_ignore_ascii_case(candidate),
            SegmentMatch::StemExactExtCi => {
                match (real.rsplit_once('.'), candidate.rsplit_once('.')) {
                    (Some((real_stem, real_ext)), Some((cand_stem, cand_ext))) => {
                        real_stem == cand_stem && real_ext.eq_ignore_ascii_case(cand_ext)
                    }
                    (None, None) => real == candidate,
                    _ => false,
                }
            }
        }
    }
}

/// Whether the path names a root module of the configuration: the file is the
/// given conventional name and its parent is the service level — `Ext` in a
/// configurator dump, `Configuration` in an EDT project — in any ASCII case,
/// with either separator. Positional on purpose: two segments from the end is
/// the service level, so an object named `Ext` or `Configuration` one level
/// higher cannot match, and neither can a bare file name.
pub fn path_is_root_module(path: &str, name: ConventionalName) -> bool {
    let mut segments = path.rsplit(['/', '\\']);
    let (Some(last), Some(prev)) = (segments.next(), segments.next()) else {
        return false;
    };
    conventional_of(last) == Some(name)
        && matches!(
            conventional_of(prev),
            Some(ConventionalName::Ext | ConventionalName::ConfigurationDir)
        )
}

/// The historical name of [`path_is_root_module`]; diagnostics call it by
/// this name.
pub fn path_ends_with_ext_child(path: &str, name: ConventionalName) -> bool {
    path_is_root_module(path, name)
}
