use crate::enums::FormType;
use crate::error::Result;
use crate::form::{
    Form, FormAttribute, FormAttributeColumn, FormElement, FormElementKind, FormEventHandler,
};
use crate::metadata_object::AttributeType;

use super::super::form::{tag_to_kind, FormMetadataInfo};
use super::super::helpers::{child_bool, child_text, child_u32, find_child};
use super::helpers::child_texts;
use super::types::parse_edt_type;

const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";

/// A managed form from its `Form.form`. The form's own name, type and uuid
/// are not in this file — they live in the owner's descriptor — so the caller
/// that knows the owner fills them in (`parse_form_from_bsl_path`).
pub(crate) fn parse_edt_form(doc: &roxmltree::Document<'_>) -> Result<Form> {
    let _span = tracing::debug_span!("parse_edt_form").entered();
    let root = doc.root_element();

    let mut elements = Vec::new();
    collect_items(root, &mut elements, None);

    let mut event_handlers = Vec::new();
    collect_handlers(root, &mut event_handlers);

    let command_handlers = root
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "formCommands")
        .filter_map(|command| find_child(command, "action"))
        .filter_map(|action| find_child(action, "handler"))
        .filter_map(|handler| child_text(handler, "name"))
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect();

    let attributes = root
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "attributes")
        .filter_map(parse_attribute)
        .collect();

    let mut form = Form::with_handlers(
        String::new(),
        FormType::Managed,
        uuid::Uuid::nil(),
        elements,
        event_handlers,
        command_handlers,
    );
    form.attributes = attributes;

    tracing::debug!(
        elements_count = form.elements().len(),
        event_handlers_count = form.event_handlers().len(),
        command_handlers_count = form.command_handlers().len(),
        attributes_count = form.attributes().len(),
        "parsed EDT form"
    );
    Ok(form)
}

/// `items` at any depth, plus the additions a control carries (`extendedTooltip`,
/// `contextMenu`, …). The `xsi:type` names the family; for a group the concrete
/// control is in `<type>`, which is the very name the configurator uses as its
/// element tag, so the shared kind table receives it unchanged.
fn collect_items(
    parent: roxmltree::Node<'_, '_>,
    out: &mut Vec<FormElement>,
    parent_id: Option<u32>,
) {
    for node in parent.children().filter(|n| n.is_element()) {
        let tag = node.tag_name().name();
        let is_item = tag == "items";
        let is_addition = matches!(
            tag,
            "extendedTooltip"
                | "contextMenu"
                | "autoCommandBar"
                | "searchStringAddition"
                | "viewStatusAddition"
                | "searchControlAddition"
        );
        if !is_item && !is_addition {
            continue;
        }
        let Some(name) = child_text(node, "name").filter(|n| !n.is_empty()) else { continue };
        let Some(id) = child_u32(node, "id") else { continue };
        let data_path = find_child(node, "dataPath")
            .map(|path| child_texts(path, "segments").join("."))
            .filter(|path| !path.is_empty());
        let kind = if is_addition {
            FormElementKind::Addition
        } else {
            let family =
                node.attribute((XSI_NS, "type")).map(|t| t.rsplit(':').next().unwrap_or(t));
            match family {
                Some("FormField") => FormElementKind::Field,
                Some("Table") => FormElementKind::Table,
                Some("Button") => FormElementKind::Button,
                Some("Decoration") => FormElementKind::Decoration,
                Some("FormGroup") => tag_to_kind(child_text(node, "type").unwrap_or("UsualGroup")),
                _ => FormElementKind::Other,
            }
        };
        out.push(FormElement::with_kind(name.to_string(), id, data_path, kind, parent_id));
        collect_items(node, out, Some(id));
    }
}

fn collect_handlers(node: roxmltree::Node<'_, '_>, out: &mut Vec<FormEventHandler>) {
    for child in node.children().filter(|n| n.is_element()) {
        if child.tag_name().name() == "handlers" {
            if let (Some(event), Some(name)) =
                (child_text(child, "event"), child_text(child, "name"))
            {
                if !name.trim().is_empty() {
                    out.push(FormEventHandler {
                        event_type: event.to_string(),
                        handler_name: name.trim().to_string(),
                    });
                }
            }
        } else {
            collect_handlers(child, out);
        }
    }
}

fn parse_attribute(node: roxmltree::Node<'_, '_>) -> Option<FormAttribute> {
    let name = child_text(node, "name").filter(|n| !n.is_empty())?.to_string();
    let attr_type = find_child(node, "valueType")
        .and_then(|t| parse_edt_type(t).ok())
        .unwrap_or(AttributeType::Unknown);
    let is_main = child_bool(node, "main");
    let columns = node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "columns")
        .filter_map(|column| {
            let name = child_text(column, "name")?.to_string();
            let attr_type = find_child(column, "valueType")
                .and_then(|t| parse_edt_type(t).ok())
                .unwrap_or(AttributeType::Unknown);
            Some(FormAttributeColumn { name, attr_type })
        })
        .collect();
    Some(FormAttribute { name, attr_type, is_main, columns })
}

/// The identity of `form_name` from its owner's descriptor: the `<forms>`
/// entry of an object, or the root of a `CommonForm` descriptor. The form
/// type is written only when it is not the managed default.
pub(crate) fn form_identity_from_owner(mdo_xml: &str, form_name: &str) -> Option<FormMetadataInfo> {
    let doc = roxmltree::Document::parse(mdo_xml).ok()?;
    let root = doc.root_element();
    let node = if root.tag_name().name() == "CommonForm" {
        (child_text(root, "name") == Some(form_name)).then_some(root)?
    } else {
        root.children().find(|n| {
            n.is_element()
                && n.tag_name().name() == "forms"
                && child_text(*n, "name") == Some(form_name)
        })?
    };
    let uuid = node.attribute("uuid").and_then(|u| u.parse().ok()).unwrap_or_else(uuid::Uuid::nil);
    let form_type =
        child_text(node, "formType").map(FormType::from_name).unwrap_or(FormType::Managed);
    Some(FormMetadataInfo { name: form_name.to_string(), form_type, uuid })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata_object::{MdoType, PlatformValueType};

    const FORM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<form:Form xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:core="http://g5.1c.ru/v8/dt/mcore" xmlns:form="http://g5.1c.ru/v8/dt/form">
  <items xsi:type="form:FormGroup">
    <name>ГруппаШапка</name>
    <id>2</id>
    <type>UsualGroup</type>
    <items xsi:type="form:FormField">
      <name>Наименование</name>
      <id>3</id>
      <dataPath xsi:type="form:DataPath"><segments>Объект.Наименование</segments></dataPath>
      <type>InputField</type>
      <handlers><event>OnChange</event><name>НаименованиеПриИзменении</name></handlers>
      <extendedTooltip><name>НаименованиеExtendedTooltip</name><id>4</id></extendedTooltip>
    </items>
  </items>
  <items xsi:type="form:Table">
    <name>Строки</name>
    <id>5</id>
    <dataPath xsi:type="form:DataPath"><segments>Объект.Строки</segments></dataPath>
    <items xsi:type="form:FormField"><name>СтрокиТовар</name><id>6</id><dataPath xsi:type="form:DataPath"><segments>Объект.Строки.Товар</segments></dataPath><type>InputField</type></items>
  </items>
  <items xsi:type="form:Button"><name>Записать</name><id>7</id><type>CommandBarButton</type></items>
  <items xsi:type="form:Decoration"><name>Надпись</name><id>8</id><type>Label</type></items>
  <items xsi:type="form:FormGroup"><name>Страницы</name><id>9</id><type>Pages</type></items>
  <formCommands><name>Заполнить</name><id>1</id><action xsi:type="form:FormCommandHandlerContainer"><handler><name>ЗаполнитьНаСервере</name></handler></action></formCommands>
  <attributes>
    <name>Объект</name>
    <id>1</id>
    <valueType><types>CatalogObject.Справочник1</types></valueType>
    <main>true</main>
  </attributes>
  <attributes>
    <name>Таблица</name>
    <id>2</id>
    <valueType><types>ValueTable</types></valueType>
    <columns><name>Колонка1</name><id>3</id><valueType><types>String</types></valueType></columns>
  </attributes>
  <handlers><event>OnOpen</event><name>ПриОткрытии</name></handlers>
  <handlers><event>OnCreateAtServer</event><name>ПриСозданииНаСервере</name></handlers>
</form:Form>"#;

    #[test]
    fn edt_form_elements_carry_kind_parent_and_data_path() {
        let doc = roxmltree::Document::parse(FORM).unwrap();
        let form = parse_edt_form(&doc).unwrap();
        let group = form.find_element("ГруппаШапка").unwrap();
        assert_eq!(group.kind, FormElementKind::UsualGroup);
        assert_eq!(group.parent_id, None);
        let field = form.find_element("Наименование").unwrap();
        assert_eq!(field.kind, FormElementKind::Field);
        assert_eq!(field.parent_id, Some(2));
        assert_eq!(field.data_path.as_deref(), Some("Объект.Наименование"));
        assert_eq!(form.find_element("Строки").unwrap().kind, FormElementKind::Table);
        assert_eq!(form.find_element("СтрокиТовар").unwrap().parent_id, Some(5));
        assert_eq!(form.find_element("Записать").unwrap().kind, FormElementKind::Button);
        assert_eq!(form.find_element("Надпись").unwrap().kind, FormElementKind::Decoration);
        assert_eq!(form.find_element("Страницы").unwrap().kind, FormElementKind::Pages);
        assert_eq!(
            form.find_element("НаименованиеExtendedTooltip").unwrap().kind,
            FormElementKind::Addition
        );
        assert_eq!(form.find_element("НаименованиеExtendedTooltip").unwrap().parent_id, Some(3));
    }

    #[test]
    fn edt_form_handlers_commands_and_attributes() {
        let doc = roxmltree::Document::parse(FORM).unwrap();
        let form = parse_edt_form(&doc).unwrap();
        let mut handlers = form.event_handler_names();
        handlers.sort();
        assert_eq!(
            handlers,
            vec!["НаименованиеПриИзменении", "ПриОткрытии", "ПриСозданииНаСервере"]
        );
        assert_eq!(
            form.event_handlers()
                .iter()
                .find(|h| h.handler_name == "ПриОткрытии")
                .unwrap()
                .event_type,
            "OnOpen"
        );
        assert_eq!(form.command_handlers(), &["ЗаполнитьНаСервере".to_string()]);
        assert!(form.is_handler("ЗаполнитьНаСервере"));
        let main = form.main_attribute().unwrap();
        assert_eq!(main.name, "Объект");
        assert_eq!(
            main.attr_type,
            AttributeType::Ref { mdo_type: MdoType::Catalog, name: "Справочник1".into() }
        );
        let table = form.find_attribute("Таблица").unwrap();
        assert_eq!(table.attr_type, AttributeType::Platform(PlatformValueType::ValueTable));
        assert_eq!(table.columns[0].name, "Колонка1");
        assert!(form.is_managed());
        assert_eq!(crate::xml_parser::parse_form_xml(FORM).unwrap(), form);
    }

    #[test]
    fn form_identity_comes_from_the_owner_descriptor() {
        let owner = r#"<mdclass:Catalog xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="31b6e35b-6ac8-4bad-8581-be21e901c63b"><name>Справочник1</name><forms uuid="a42b835f-e936-4ec5-9d00-8cf0b095b0bb"><name>ФормаЭлемента</name></forms><forms uuid="fd39ec4f-11c7-435e-b234-8f52d8bb9f70"><name>ФормаСписка</name><formType>Ordinary</formType></forms></mdclass:Catalog>"#;
        let info = form_identity_from_owner(owner, "ФормаСписка").unwrap();
        assert_eq!(info.name, "ФормаСписка");
        assert_eq!(info.form_type, FormType::Ordinary);
        assert_eq!(info.uuid.to_string(), "fd39ec4f-11c7-435e-b234-8f52d8bb9f70");
        assert_eq!(
            form_identity_from_owner(owner, "ФормаЭлемента").unwrap().form_type,
            FormType::Managed
        );
        assert!(form_identity_from_owner(owner, "Нет").is_none());
        let common = r#"<mdclass:CommonForm xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="1f3057c2-135f-44b2-9f86-34481fbc5596"><name>ОбщаяФорма1</name></mdclass:CommonForm>"#;
        assert_eq!(
            form_identity_from_owner(common, "ОбщаяФорма1").unwrap().uuid.to_string(),
            "1f3057c2-135f-44b2-9f86-34481fbc5596"
        );
        assert!(form_identity_from_owner(common, "Другая").is_none());
    }
}
