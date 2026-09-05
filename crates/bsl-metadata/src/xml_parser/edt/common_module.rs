use crate::common_module::CommonModule;
use crate::enums::ReturnValueReuse;
use crate::error::Result;

use super::super::helpers::{child_bool, child_text};
use super::helpers::{object_belonging_of, root_uuid};

/// A common module from its EDT descriptor. EDT writes only the flags that
/// are set (`<server>true</server>`); an absent flag is `false`, which is what
/// `child_bool` answers for a missing element.
pub(crate) fn parse_edt_common_module(doc: &roxmltree::Document<'_>) -> Result<CommonModule> {
    let root = doc.root_element();
    let mut builder = CommonModule::builder()
        .uuid(root_uuid(root, "common module")?)
        .name(child_text(root, "name").unwrap_or(""))
        .server(child_bool(root, "server"))
        .global(child_bool(root, "global"))
        .client_managed_application(child_bool(root, "clientManagedApplication"))
        .client_ordinary_application(child_bool(root, "clientOrdinaryApplication"))
        .external_connection(child_bool(root, "externalConnection"))
        .server_call(child_bool(root, "serverCall"))
        .privileged(child_bool(root, "privileged"))
        .return_values_reuse(ReturnValueReuse::from_name(
            child_text(root, "returnValuesReuse").unwrap_or(""),
        ))
        .object_belonging(object_belonging_of(root));
    if let Some(comment) = child_text(root, "comment") {
        builder = builder.comment(comment);
    }
    Ok(builder.build())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::ObjectBelonging;
    use crate::traits::MdObject;

    #[test]
    fn edt_common_module_flags_default_to_false_when_absent() {
        let xml = r#"<mdclass:CommonModule xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="75e77f99-b9d1-4bc0-93aa-c77a93d760d3"><name>ОбщегоНазначения</name><comment>Серверные</comment><server>true</server><externalConnection>true</externalConnection><clientOrdinaryApplication>true</clientOrdinaryApplication><returnValuesReuse>DuringSession</returnValuesReuse></mdclass:CommonModule>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let m = parse_edt_common_module(&doc).unwrap();
        assert_eq!(m.name(), "ОбщегоНазначения");
        assert!(m.is_server() && m.is_external_connection() && m.is_client_ordinary_application());
        assert!(
            !m.is_global()
                && !m.is_client_managed_application()
                && !m.is_server_call()
                && !m.is_privileged()
        );
        assert_eq!(m.return_values_reuse(), ReturnValueReuse::DuringSession);
        assert_eq!(m.comment(), Some("Серверные"));
        assert_eq!(m.object_belonging(), ObjectBelonging::Own);
        assert_eq!(crate::xml_parser::parse_common_module_xml(xml).unwrap(), m);
    }

    #[test]
    fn edt_adopted_common_module_is_marked() {
        let xml = r#"<mdclass:CommonModule xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" xmlns:mdclassExtension="http://g5.1c.ru/v8/dt/metadata/mdclass/extension" uuid="da692fc6-2556-467b-80bd-4f80afa77325" extendedConfigurationObject="27651e0e-d560-434b-a49c-f473a5b20c4d"><name>Переопределяемый</name><objectBelonging>Adopted</objectBelonging><extension xsi:type="mdclassExtension:CommonModuleExtension"><server>Checked</server></extension><server>true</server></mdclass:CommonModule>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let m = parse_edt_common_module(&doc).unwrap();
        assert_eq!(m.object_belonging(), ObjectBelonging::Adopted);
        assert!(m.is_server());
    }
}
