use std::str::FromStr;

use crate::error::Result;
use crate::metadata_object::{
    Attribute, AttributeType, EnumValue, MdoType, MetadataObject, PredefinedItem,
};
use crate::tabular_section::{TabularSection, TabularSectionAttribute};

use super::super::catalog::{
    build_calculation_type_dependency_sections, build_ext_dimension_types_section_from_type,
    parse_code_series,
};
use super::super::helpers::{child_text, find_child, parse_uuid};
use super::super::standard_attributes::{
    add_business_process_standard_attributes, add_catalog_standard_attributes,
    add_chart_of_accounts_standard_attributes, add_chart_of_calculation_types_standard_attributes,
    add_chart_of_characteristic_types_standard_attributes, add_document_standard_attributes,
    add_exchange_plan_standard_attributes, add_task_standard_attributes,
};
use super::helpers::{child_texts, object_belonging_of, root_uuid};
use super::properties::edt_properties;
use super::types::parse_edt_type;

/// One object of the catalog family (catalogs, documents, charts, business
/// processes, tasks, exchange plans, data processors, reports) from its EDT
/// descriptor. Children are direct repeated elements of the root — `attributes`,
/// `tabularSections`, `predefined` — instead of a `ChildObjects` block, and the
/// standard attributes come from the same tables the configurator reader uses.
pub(crate) fn parse_edt_metadata_object(
    doc: &roxmltree::Document<'_>,
    mdo_type: MdoType,
) -> Result<MetadataObject> {
    let root = doc.root_element();
    let properties = edt_properties(root);

    let mut attributes = Vec::new();
    match mdo_type {
        MdoType::Catalog => {
            add_catalog_standard_attributes(&mut attributes, &properties, mdo_type);
        }
        MdoType::Document => {
            add_document_standard_attributes(&mut attributes, &properties, mdo_type);
        }
        MdoType::BusinessProcess => {
            add_business_process_standard_attributes(&mut attributes, &properties, mdo_type);
        }
        MdoType::Task => {
            add_task_standard_attributes(&mut attributes, &properties, mdo_type);
        }
        MdoType::ExchangePlan => {
            add_exchange_plan_standard_attributes(&mut attributes, &properties, mdo_type);
        }
        MdoType::ChartOfCharacteristicTypes => {
            add_chart_of_characteristic_types_standard_attributes(
                &mut attributes,
                &properties,
                mdo_type,
            );
        }
        MdoType::ChartOfAccounts => {
            add_chart_of_accounts_standard_attributes(&mut attributes, &properties, mdo_type);
        }
        MdoType::ChartOfCalculationTypes => {
            add_chart_of_calculation_types_standard_attributes(
                &mut attributes,
                &properties,
                mdo_type,
            );
        }
        _ => {}
    }

    let mut ext_dimension_flags = Vec::new();
    let mut tabular_sections = Vec::new();
    for child in root.children().filter(|n| n.is_element()) {
        match child.tag_name().name() {
            // A task's addressing attribute and a chart of accounts' accounting
            // flag are members of the object row, as they are in the
            // configurator's dump.
            "attributes" | "addressingAttributes" | "accountingFlags" => {
                attributes.push(parse_attribute(child)?);
            }
            "extDimensionAccountingFlags" => ext_dimension_flags.push(parse_attribute(child)?),
            "tabularSections" => tabular_sections.push(parse_tabular_section(child)?),
            _ => {}
        }
    }

    if mdo_type == MdoType::ChartOfAccounts {
        let ext_dim_kind_type = child_text(root, "extDimensionTypes")
            .and_then(|raw| {
                let (prefix, name) = raw.split_once('.')?;
                let mdo_type = MdoType::from_str(prefix).ok()?;
                Some(AttributeType::Ref { mdo_type, name: name.to_string() })
            })
            .unwrap_or(AttributeType::Unknown);
        tabular_sections.push(build_ext_dimension_types_section_from_type(
            ext_dim_kind_type,
            ext_dimension_flags,
        ));
    }
    if mdo_type == MdoType::ChartOfCalculationTypes && properties.depends_on_calculation_types() {
        tabular_sections.extend(build_calculation_type_dependency_sections(&properties.name));
    }

    let mut mdo = MetadataObject::new(mdo_type, properties.name.clone());
    mdo.set_uuid(root_uuid(root, "MDO root")?);
    if mdo_type == MdoType::Document {
        mdo.set_register_records(
            child_texts(root, "registerRecords")
                .into_iter()
                .filter_map(|raw| {
                    let (prefix, name) = raw.split_once('.')?;
                    let mdo_type = MdoType::from_str(prefix).ok()?;
                    (mdo_type.is_register() && !name.is_empty())
                        .then(|| (mdo_type, name.to_string()))
                })
                .collect(),
        );
    }
    for attr in attributes {
        mdo.add_attribute(attr);
    }
    for ts in tabular_sections {
        mdo.add_tabular_section(ts);
    }
    mdo.set_check_unique(properties.check_unique);
    if let Some(code_series) = &properties.code_series {
        mdo.set_code_series(parse_code_series(code_series));
    }
    if let Some(predefined) = find_child(root, "predefined") {
        mdo.predefined_items = parse_edt_predefined(predefined);
    }
    mdo.object_belonging = object_belonging_of(root);

    tracing::debug!(
        mdo_name = %mdo.name,
        mdo_type = ?mdo.mdo_type,
        attributes = mdo.attributes.len(),
        tabular_sections = mdo.tabular_sections.len(),
        predefined = mdo.predefined_items.len(),
        "parsed EDT metadata object"
    );
    Ok(mdo)
}

fn parse_attribute(node: roxmltree::Node<'_, '_>) -> Result<Attribute> {
    let name = child_text(node, "name").unwrap_or("").to_string();
    let attr_type = match find_child(node, "type") {
        Some(t) => parse_edt_type(t)?,
        None => AttributeType::Unknown,
    };
    Ok(Attribute { name, name_en: None, attr_type })
}

fn parse_tabular_section(node: roxmltree::Node<'_, '_>) -> Result<TabularSection> {
    let uuid = parse_uuid(node.attribute("uuid").unwrap_or(""), "tabular section")?;
    let mut section = TabularSection::new(uuid, child_text(node, "name").unwrap_or(""));
    let mut columns = Vec::new();
    for attr in node.children().filter(|n| n.is_element() && n.tag_name().name() == "attributes") {
        let attr_uuid =
            parse_uuid(attr.attribute("uuid").unwrap_or(""), "tabular section attribute")?;
        let parsed = parse_attribute(attr)?;
        columns.push(TabularSectionAttribute::new(attr_uuid, parsed.name, parsed.attr_type));
    }
    section.set_attributes(columns);
    Ok(section)
}

/// `<predefined><items id><name>…<items>…</items></items>`: nested items are
/// children of their parent item, as `ChildItems` are in `Predefined.xml`.
pub(crate) fn parse_edt_predefined(node: roxmltree::Node<'_, '_>) -> Vec<PredefinedItem> {
    let mut items = Vec::new();
    collect_items(node, &mut items);
    items
}

fn collect_items(node: roxmltree::Node<'_, '_>, items: &mut Vec<PredefinedItem>) {
    for item in node.children().filter(|n| n.is_element() && n.tag_name().name() == "items") {
        let name = child_text(item, "name").unwrap_or("").to_string();
        if !name.is_empty() {
            items.push(PredefinedItem {
                name,
                name_en: None,
                uuid: item.attribute("id").unwrap_or("").to_string(),
            });
        }
        collect_items(item, items);
    }
}

pub(crate) fn parse_edt_enum(doc: &roxmltree::Document<'_>) -> Result<MetadataObject> {
    let root = doc.root_element();
    let mut mdo = MetadataObject::new(MdoType::Enum, child_text(root, "name").unwrap_or(""));
    mdo.set_uuid(root_uuid(root, "Enum root")?);
    mdo.enum_values = root
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "enumValues")
        .map(|v| EnumValue {
            name: child_text(v, "name").unwrap_or("").to_string(),
            name_en: None,
            uuid: v.attribute("uuid").unwrap_or("").to_string(),
        })
        .collect();
    mdo.object_belonging = object_belonging_of(root);
    Ok(mdo)
}

pub(crate) fn parse_edt_constant(doc: &roxmltree::Document<'_>) -> Result<MetadataObject> {
    let root = doc.root_element();
    let mut mdo = MetadataObject::new(MdoType::Constant, child_text(root, "name").unwrap_or(""));
    mdo.set_uuid(root_uuid(root, "Constant root")?);
    if let Some(t) = find_child(root, "type") {
        mdo.set_constant_type(parse_edt_type(t)?);
    }
    mdo.object_belonging = object_belonging_of(root);
    Ok(mdo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::ObjectBelonging;

    const CATALOG: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<mdclass:Catalog xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:core="http://g5.1c.ru/v8/dt/mcore" xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="31b6e35b-6ac8-4bad-8581-be21e901c63b">
  <name>Справочник1</name>
  <owners>Catalog.Владельцы</owners>
  <hierarchical>true</hierarchical>
  <codeLength>9</codeLength>
  <descriptionLength>100</descriptionLength>
  <codeType>Number</codeType>
  <codeSeries>WithinOwnerSubordination</codeSeries>
  <checkUnique>true</checkUnique>
  <attributes uuid="56db42c8-34e7-466e-a3fe-c7a59ab68efa">
    <name>Реквизит1</name>
    <type><types>CatalogRef.Владельцы</types></type>
  </attributes>
  <attributes uuid="90a6e696-4ebc-4edb-acbe-13ee9084a388">
    <name>Сумма</name>
    <type><types>Number</types><numberQualifiers><precision>15</precision><scale>2</scale></numberQualifiers></type>
  </attributes>
  <tabularSections uuid="1189fa57-64e3-481e-8764-83a1d85f6156">
    <name>Строки</name>
    <attributes uuid="8abe9e12-c3df-47b7-b0f1-7fd307ee3fa9">
      <name>Товар</name>
      <type><types>CatalogRef.Справочник1</types></type>
    </attributes>
  </tabularSections>
  <predefined>
    <items id="2b56f8fc-a0e3-43b1-9727-8d68c6849d59">
      <name>Корень</name>
      <items id="2ae45e46-1458-4fe1-8f3a-8e284c18b1c1">
        <name>Лист</name>
      </items>
    </items>
  </predefined>
</mdclass:Catalog>"#;

    #[test]
    fn edt_catalog_carries_attributes_sections_predefined_and_standard_attributes() {
        let doc = roxmltree::Document::parse(CATALOG).unwrap();
        let mdo = parse_edt_metadata_object(&doc, MdoType::Catalog).unwrap();
        assert_eq!(mdo.name, "Справочник1");
        assert_eq!(mdo.uuid().unwrap().to_string(), "31b6e35b-6ac8-4bad-8581-be21e901c63b");
        assert_eq!(mdo.object_belonging, ObjectBelonging::Own);
        assert!(mdo.check_unique);
        assert_eq!(mdo.code_series, crate::enums::CodeSeries::WithinOwnerSubordination);
        let names: Vec<&str> = mdo.attributes.iter().map(|a| a.name.as_str()).collect();
        for standard in [
            "Ссылка",
            "Код",
            "Наименование",
            "Владелец",
            "Родитель",
            "ЭтоГруппа",
            "ПометкаУдаления",
        ] {
            assert!(names.contains(&standard), "{standard} missing: {names:?}");
        }
        let code = mdo.attributes.iter().find(|a| a.name == "Код").unwrap();
        assert_eq!(code.attr_type, AttributeType::Number { precision: 9, scale: 0 });
        let owner = mdo.attributes.iter().find(|a| a.name == "Владелец").unwrap();
        assert_eq!(
            owner.attr_type,
            AttributeType::Ref { mdo_type: MdoType::Catalog, name: "Владельцы".into() }
        );
        let sum = mdo.attributes.iter().find(|a| a.name == "Сумма").unwrap();
        assert_eq!(sum.attr_type, AttributeType::Number { precision: 15, scale: 2 });
        assert_eq!(mdo.tabular_sections.len(), 1);
        assert_eq!(mdo.tabular_sections[0].name(), "Строки");
        assert_eq!(mdo.tabular_sections[0].attributes()[0].name(), "Товар");
        let predefined: Vec<&str> = mdo.predefined_items.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(predefined, vec!["Корень", "Лист"]);
    }

    #[test]
    fn edt_document_reads_register_records_and_number_type() {
        let xml = r#"<mdclass:Document xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="d6587a61-ed0e-40e0-9c89-c4b7a3946e7f">
  <name>Документ1</name>
  <numberType>String</numberType>
  <numberLength>11</numberLength>
  <registerRecords>InformationRegister.РегистрСведений1</registerRecords>
  <registerRecords>AccumulationRegister.РегистрНакопления1</registerRecords>
  <registerRecords>Catalog.НеРегистр</registerRecords>
</mdclass:Document>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let mdo = parse_edt_metadata_object(&doc, MdoType::Document).unwrap();
        assert_eq!(mdo.register_records().len(), 2);
        assert_eq!(
            mdo.register_records()[0],
            (MdoType::InformationRegister, "РегистрСведений1".to_string())
        );
        let number = mdo.attributes.iter().find(|a| a.name == "Номер").unwrap();
        assert_eq!(number.attr_type, AttributeType::String { length: Some(11) });
        assert!(mdo.attributes.iter().any(|a| a.name == "Проведен"));
    }

    #[test]
    fn edt_adopted_object_is_marked() {
        let xml = r#"<mdclass:Catalog xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" xmlns:mdclassExtension="http://g5.1c.ru/v8/dt/metadata/mdclass/extension" uuid="dbbdb9d5-0d40-4195-8812-31ea0b0e85aa" extendedConfigurationObject="221738ec-6ae2-40e4-bf80-bf4e019c9b50">
  <name>Товары</name>
  <objectBelonging>Adopted</objectBelonging>
  <extension xsi:type="mdclassExtension:CatalogExtension"><extendedConfigurationObject>Checked</extendedConfigurationObject></extension>
  <attributes uuid="3040ed88-0ce7-43bc-a187-c8f9e5c577fe"><name>Добавленный</name><type><types>String</types></type></attributes>
</mdclass:Catalog>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let mdo = parse_edt_metadata_object(&doc, MdoType::Catalog).unwrap();
        assert_eq!(mdo.object_belonging, ObjectBelonging::Adopted);
        assert!(mdo.attributes.iter().any(|a| a.name == "Добавленный"));
    }

    #[test]
    fn edt_chart_of_calculation_types_gets_dependency_sections() {
        let xml = r#"<mdclass:ChartOfCalculationTypes xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="a0b1c2d3-0000-4000-8000-000000000401">
  <name>Начисления</name>
  <codeLength>5</codeLength>
  <descriptionLength>50</descriptionLength>
  <dependenceOnCalculationTypes>OnActionPeriod</dependenceOnCalculationTypes>
  <actionPeriodUse>true</actionPeriodUse>
</mdclass:ChartOfCalculationTypes>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let mdo = parse_edt_metadata_object(&doc, MdoType::ChartOfCalculationTypes).unwrap();
        let sections: Vec<&str> = mdo.tabular_sections.iter().map(|s| s.name()).collect();
        assert!(sections.contains(&"БазовыеВидыРасчета"), "{sections:?}");
    }

    #[test]
    fn edt_enum_and_constant() {
        let e = roxmltree::Document::parse(r#"<mdclass:Enum xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="d1a0c1ac-4880-4e3b-a380-85a02fcfc16d"><name>Перечисление1</name><enumValues uuid="0a425009-6ddc-4bc0-8d59-7dbbd1b30772"><name>Значение1</name></enumValues><enumValues uuid="6777a3c5-9dd9-4eb2-99b1-5cc457aa5d21"><name>Значение2</name></enumValues></mdclass:Enum>"#).unwrap();
        let mdo = parse_edt_enum(&e).unwrap();
        assert_eq!(mdo.mdo_type, MdoType::Enum);
        assert_eq!(
            mdo.enum_values.iter().map(|v| v.name.as_str()).collect::<Vec<_>>(),
            vec!["Значение1", "Значение2"]
        );
        assert_eq!(mdo.enum_values[0].uuid, "0a425009-6ddc-4bc0-8d59-7dbbd1b30772");
        let c = roxmltree::Document::parse(r#"<mdclass:Constant xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="9893e2d6-f3f8-4d73-bb06-19bf26d216ab"><name>Константа1</name><type><types>Boolean</types></type></mdclass:Constant>"#).unwrap();
        let mdo = parse_edt_constant(&c).unwrap();
        assert_eq!(mdo.mdo_type, MdoType::Constant);
        assert_eq!(mdo.constant_type, Some(AttributeType::Boolean));
    }

    #[test]
    fn public_parsers_dispatch_on_the_namespace() {
        let mdo = crate::xml_parser::parse_catalog_xml(CATALOG).unwrap();
        assert_eq!(mdo.name, "Справочник1");
        assert!(crate::xml_parser::parse_catalog_xml(r#"<other:Catalog xmlns:other="urn:x"/>"#)
            .is_err());
        let e = crate::xml_parser::parse_enum_xml(r#"<mdclass:Enum xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="d1a0c1ac-4880-4e3b-a380-85a02fcfc16d"><name>П</name></mdclass:Enum>"#).unwrap();
        assert_eq!(e.name, "П");
        let c = crate::xml_parser::parse_constant_xml(r#"<mdclass:Constant xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="9893e2d6-f3f8-4d73-bb06-19bf26d216ab"><name>К</name></mdclass:Constant>"#).unwrap();
        assert_eq!(c.name, "К");
    }
}
