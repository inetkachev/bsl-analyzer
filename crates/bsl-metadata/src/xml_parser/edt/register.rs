use crate::dimension::Dimension;
use crate::error::Result;
use crate::metadata_object::MdoType;
use crate::register::{Register, RegisterAttribute, RegisterResource};

use super::super::helpers::{child_bool, child_text, find_child, parse_uuid};
use super::super::register::{parse_periodicity, parse_register_type};
use super::super::standard_attributes::{
    add_accumulation_register_standard_attrs, add_information_register_standard_attrs,
};
use super::helpers::{object_belonging_of, root_uuid};
use super::types::parse_edt_type;

/// A register from its EDT descriptor: `dimensions`, `resources` and
/// `attributes` are repeated root children, periodicity and register type are
/// camelCase properties of the root.
pub(crate) fn parse_edt_register(
    doc: &roxmltree::Document<'_>,
    mdo_type: MdoType,
) -> Result<Register> {
    let root = doc.root_element();
    let uuid = root_uuid(root, "register")?;
    let object_name = child_text(root, "name").unwrap_or("").to_string();
    let _span = tracing::debug_span!("parse_edt_register", ?mdo_type, %object_name).entered();

    let periodicity_str = child_text(root, "informationRegisterPeriodicity").map(str::to_string);
    let register_type_str = child_text(root, "registerType").map(str::to_string);

    let mut attributes = Vec::new();
    match mdo_type {
        MdoType::InformationRegister => add_information_register_standard_attrs(
            &mut attributes,
            &object_name,
            periodicity_str.as_deref(),
        ),
        MdoType::AccumulationRegister => {
            add_accumulation_register_standard_attrs(&mut attributes, &object_name)
        }
        _ => {}
    }

    let mut dimensions = Vec::new();
    let mut resources = Vec::new();
    for child in root.children().filter(|n| n.is_element()) {
        match child.tag_name().name() {
            "dimensions" => dimensions.push(parse_dimension(child)?),
            "resources" => resources.push(parse_resource(child)?),
            "attributes" => attributes.push(parse_attribute(child)?),
            _ => {}
        }
    }

    Ok(Register::builder()
        .uuid(uuid)
        .name(object_name)
        .mdo_type(mdo_type)
        .dimensions(dimensions)
        .resources(resources)
        .attributes(attributes)
        .periodicity(parse_periodicity(periodicity_str.as_deref(), mdo_type))
        .register_type(parse_register_type(register_type_str.as_deref(), mdo_type))
        .enable_totals_slice_first(child_bool(root, "enableTotalsSliceFirst"))
        .enable_totals_slice_last(child_bool(root, "enableTotalsSliceLast"))
        .object_belonging(object_belonging_of(root))
        .build())
}

fn parse_dimension(node: roxmltree::Node<'_, '_>) -> Result<Dimension> {
    let uuid = parse_uuid(node.attribute("uuid").unwrap_or(""), "dimension")?;
    let mut dimension = Dimension::builder()
        .uuid(uuid)
        .name(child_text(node, "name").unwrap_or(""))
        .deny_incomplete_values(child_bool(node, "denyIncompleteValues"))
        .master(child_bool(node, "master"))
        .indexing(child_text(node, "indexing").unwrap_or(""))
        .build();
    if let Some(type_node) = find_child(node, "type") {
        let dim_type = parse_edt_type(type_node)?;
        dimension.set_type_str(format!("{}", dim_type));
        dimension.set_attr_type(dim_type);
    }
    Ok(dimension)
}

fn parse_resource(node: roxmltree::Node<'_, '_>) -> Result<RegisterResource> {
    let uuid = parse_uuid(node.attribute("uuid").unwrap_or(""), "resource")?;
    let mut resource = RegisterResource::new(uuid, child_text(node, "name").unwrap_or(""));
    if let Some(type_node) = find_child(node, "type") {
        let resource_type = parse_edt_type(type_node)?;
        resource.set_type_str(format!("{}", resource_type));
        resource.set_attr_type(resource_type);
    }
    Ok(resource)
}

fn parse_attribute(node: roxmltree::Node<'_, '_>) -> Result<RegisterAttribute> {
    let uuid = parse_uuid(node.attribute("uuid").unwrap_or(""), "attribute")?;
    let mut attribute = RegisterAttribute::new(uuid, child_text(node, "name").unwrap_or(""));
    if let Some(type_node) = find_child(node, "type") {
        let attr_type = parse_edt_type(type_node)?;
        attribute.set_type_str(format!("{}", attr_type));
        attribute.set_attr_type(attr_type);
    }
    Ok(attribute)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::ObjectBelonging;
    use crate::register::{AccumulationRegisterType, RegisterPeriodicity};
    use crate::traits::MdObject;

    #[test]
    fn edt_information_register_reads_dimensions_resources_attributes_and_period() {
        let xml = r#"<mdclass:InformationRegister xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="ee784ea1-aa26-47fa-9090-f0d3606c21b5">
  <name>РегистрСведений1</name>
  <informationRegisterPeriodicity>Day</informationRegisterPeriodicity>
  <writeMode>RecorderSubordinate</writeMode>
  <dimensions uuid="e78d8b9f-e50c-42f3-aa4d-9f4b6522afd3"><name>Измерение1</name><type><types>CatalogRef.Справочник1</types></type><denyIncompleteValues>true</denyIncompleteValues><master>true</master><indexing>Index</indexing></dimensions>
  <resources uuid="e78d8b9f-e50c-42f3-aa4d-9f4b6522afd4"><name>Ресурс1</name><type><types>Number</types><numberQualifiers><precision>10</precision></numberQualifiers></type></resources>
  <attributes uuid="e78d8b9f-e50c-42f3-aa4d-9f4b6522afd5"><name>Реквизит1</name><type><types>String</types></type></attributes>
</mdclass:InformationRegister>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let reg = parse_edt_register(&doc, MdoType::InformationRegister).unwrap();
        assert_eq!(reg.name(), "РегистрСведений1");
        assert_eq!(reg.periodicity(), Some(RegisterPeriodicity::Day));
        assert_eq!(reg.dimensions().len(), 1);
        assert!(reg.dimensions()[0].is_master());
        assert!(reg.dimensions()[0].is_deny_incomplete_values());
        assert_eq!(reg.dimensions()[0].indexing(), "Index");
        assert_eq!(reg.resources()[0].name(), "Ресурс1");
        let names: Vec<&str> = reg.attributes().iter().map(|a| a.name()).collect();
        assert!(names.contains(&"Период"), "{names:?}");
        assert!(names.contains(&"Регистратор"), "{names:?}");
        assert!(names.contains(&"Реквизит1"), "{names:?}");
        assert_eq!(reg.object_belonging(), ObjectBelonging::Own);
        let via_public = crate::xml_parser::parse_information_register_xml(xml).unwrap();
        assert_eq!(via_public, reg);
    }

    #[test]
    fn edt_accumulation_register_reads_register_type() {
        let xml = r#"<mdclass:AccumulationRegister xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="ee784ea1-aa26-47fa-9090-f0d3606c21b6"><name>РегистрНакопления1</name><objectBelonging>Adopted</objectBelonging><registerType>Turnovers</registerType></mdclass:AccumulationRegister>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let reg = parse_edt_register(&doc, MdoType::AccumulationRegister).unwrap();
        assert_eq!(reg.register_type(), Some(AccumulationRegisterType::Turnovers));
        assert!(reg.attributes().iter().any(|a| a.name() == "Период"));
        assert_eq!(reg.object_belonging(), ObjectBelonging::Adopted);
    }
}
