use crate::enums::ObjectBelonging;
use crate::error::Result;
use uuid::Uuid;

use super::super::helpers::{child_text, parse_uuid};

/// The namespace of every `.mdo` root element.
pub(crate) const MDCLASS_NS: &str = "http://g5.1c.ru/v8/dt/metadata/mdclass";
/// The namespace of a `Form.form` root element.
pub(crate) const FORM_NS: &str = "http://g5.1c.ru/v8/dt/form";

pub(crate) fn is_edt_document(doc: &roxmltree::Document<'_>) -> bool {
    doc.root_element().tag_name().namespace() == Some(MDCLASS_NS)
}

pub(crate) fn is_edt_form_document(doc: &roxmltree::Document<'_>) -> bool {
    doc.root_element().tag_name().namespace() == Some(FORM_NS)
}

/// Every text of the repeated child `tag`: EDT writes a multi-valued property
/// as one element per value (`<owners>…</owners><owners>…</owners>`).
pub(crate) fn child_texts<'a>(node: roxmltree::Node<'a, 'a>, tag: &str) -> Vec<&'a str> {
    node.children()
        .filter(|n| n.is_element() && n.tag_name().name() == tag)
        .filter_map(|n| n.text())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

/// `<objectBelonging>` of an object root; absent on everything a configuration
/// owns, `Adopted` on what an extension borrowed.
pub(crate) fn object_belonging_of(node: roxmltree::Node<'_, '_>) -> ObjectBelonging {
    child_text(node, "objectBelonging").map(ObjectBelonging::from_name).unwrap_or_default()
}

pub(crate) fn root_uuid(node: roxmltree::Node<'_, '_>, context: &str) -> Result<Uuid> {
    parse_uuid(node.attribute("uuid").unwrap_or(""), context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edt_document_detection() {
        let edt = roxmltree::Document::parse(
            r#"<mdclass:Catalog xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="a"/>"#,
        )
        .unwrap();
        let designer = roxmltree::Document::parse(
            r#"<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses"><Catalog/></MetaDataObject>"#,
        )
        .unwrap();
        let form =
            roxmltree::Document::parse(r#"<form:Form xmlns:form="http://g5.1c.ru/v8/dt/form"/>"#)
                .unwrap();
        assert!(is_edt_document(&edt));
        assert!(!is_edt_document(&designer));
        assert!(!is_edt_document(&form));
        assert!(is_edt_form_document(&form));
        assert!(!is_edt_form_document(&edt));
    }

    #[test]
    fn repeated_children_and_belonging() {
        let doc = roxmltree::Document::parse(
            r#"<mdclass:Catalog xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="dbbdb9d5-0d40-4195-8812-31ea0b0e85aa"><owners>Catalog.А</owners><owners> Catalog.Б </owners><owners></owners><objectBelonging>Adopted</objectBelonging></mdclass:Catalog>"#,
        )
        .unwrap();
        let root = doc.root_element();
        assert_eq!(child_texts(root, "owners"), vec!["Catalog.А", "Catalog.Б"]);
        assert_eq!(object_belonging_of(root), ObjectBelonging::Adopted);
        assert_eq!(
            root_uuid(root, "catalog").unwrap().to_string(),
            "dbbdb9d5-0d40-4195-8812-31ea0b0e85aa"
        );
        let own = roxmltree::Document::parse(
            r#"<mdclass:Catalog xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass"/>"#,
        )
        .unwrap();
        assert_eq!(object_belonging_of(own.root_element()), ObjectBelonging::Own);
        assert!(root_uuid(own.root_element(), "catalog").is_err());
    }
}
