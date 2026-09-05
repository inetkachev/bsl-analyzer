use crate::error::Result;
use crate::metadata_object::AttributeType;

use super::super::helpers::{child_text, child_u32, find_child};
use super::super::type_parser::{parse_type_tokens, TypeQualifiers};
use super::helpers::child_texts;

/// EDT's `<type>`: `<types>` per member, qualifiers as sibling elements. The
/// tokens are the configurator's own without the namespace prefixes
/// (`CatalogRef.Имя` for `cfg:CatalogRef.Имя`, `String` for `xs:string`), so
/// each is mapped back onto the prefixed spelling and handed to the one table
/// that already knows every kind of reference.
pub(crate) fn parse_edt_type(type_node: roxmltree::Node<'_, '_>) -> Result<AttributeType> {
    let number_qualifiers = find_child(type_node, "numberQualifiers");
    let qualifiers = TypeQualifiers {
        string_length: find_child(type_node, "stringQualifiers")
            .and_then(|q| child_u32(q, "length")),
        number_digits: number_qualifiers
            .and_then(|q| child_u32(q, "precision"))
            .map(|v| v.min(u8::MAX as u32) as u8),
        number_fraction_digits: number_qualifiers
            .and_then(|q| child_u32(q, "scale"))
            .map(|v| v.min(u8::MAX as u32) as u8),
        date_fractions: find_child(type_node, "dateQualifiers")
            .and_then(|q| child_text(q, "dateFractions"))
            .map(str::to_string),
    };

    let mut types: Vec<String> = Vec::new();
    let mut type_sets: Vec<String> = Vec::new();
    for token in child_texts(type_node, "types") {
        match classify_token(token) {
            Token::Type(t) => types.push(t),
            Token::TypeSet(t) => type_sets.push(t),
        }
    }
    let types: Vec<&str> = types.iter().map(String::as_str).collect();
    let type_sets: Vec<&str> = type_sets.iter().map(String::as_str).collect();
    parse_type_tokens(&types, &type_sets, &qualifiers)
}

enum Token {
    /// A `<Type>` of the configurator: one concrete type.
    Type(String),
    /// A `<TypeSet>` of the configurator: a whole family of references.
    TypeSet(String),
}

fn classify_token(token: &str) -> Token {
    match token {
        "String" => return Token::Type("xs:string".into()),
        "Number" => return Token::Type("xs:decimal".into()),
        "Boolean" => return Token::Type("xs:boolean".into()),
        "Date" => return Token::Type("xs:dateTime".into()),
        "UUID" => return Token::Type("v8:UUID".into()),
        "ValueStorage" => return Token::Type("v8:ValueStorage".into()),
        "AnyRef" => return Token::TypeSet("cfg:AnyIBRef".into()),
        "BusinessProcessRoutePointRef" => {
            return Token::TypeSet("cfg:BusinessProcessRoutePointRef".into());
        }
        _ => {}
    }
    if let Some(rest) = token.strip_prefix("DefinedType.") {
        return Token::TypeSet(format!("cfg:DefinedType.{rest}"));
    }
    if let Some(rest) = token.strip_prefix("Characteristic.") {
        return Token::TypeSet(format!("cfg:Characteristic.{rest}"));
    }
    if let Some(prefixed) =
        PLATFORM_TOKENS.iter().find_map(|(edt, prefixed)| (*edt == token).then_some(*prefixed))
    {
        return Token::Type(prefixed.into());
    }
    let (kind, _) = token.split_once('.').unwrap_or((token, ""));
    let is_configuration_kind = kind.ends_with("Ref")
        || kind.ends_with("Object")
        || kind.ends_with("RecordSet")
        || kind.ends_with("RecordManager")
        || kind.ends_with("ValueManager")
        || kind.ends_with("Manager");
    if is_configuration_kind {
        // `CatalogRef` without an object name is the whole kind, which the
        // configurator writes as a type set.
        return if token.contains('.') {
            Token::Type(format!("cfg:{token}"))
        } else {
            Token::TypeSet(format!("cfg:{token}"))
        };
    }
    // A platform type by its English name: the catalogue resolves the local
    // name after the prefix, whatever the prefix is.
    Token::Type(format!("v8:{token}"))
}

/// EDT platform tokens that the configurator writes under a namespace prefix
/// the type table keys on.
const PLATFORM_TOKENS: &[(&str, &str)] = &[
    ("ValueList", "v8:ValueListType"),
    ("ValueTable", "v8:ValueTable"),
    ("ValueTree", "v8:ValueTree"),
    ("StandardPeriod", "v8:StandardPeriod"),
    ("StandardBeginningDate", "v8:StandardBeginningDate"),
    ("TypeDescription", "v8:TypeDescription"),
    ("FixedStructure", "v8:FixedStructure"),
    ("FixedArray", "v8:FixedArray"),
    ("FixedMap", "v8:FixedMap"),
    ("Type", "v8:Type"),
    ("Null", "v8:Null"),
    ("FormattedString", "v8ui:FormattedString"),
    ("Picture", "v8ui:Picture"),
    ("Color", "v8ui:Color"),
    ("Font", "v8ui:Font"),
    ("SpreadsheetDocument", "mxl:SpreadsheetDocument"),
    ("FormattedDocument", "fd:FormattedDocument"),
    ("Chart", "d7p1:Chart"),
    ("GanttChart", "d5p1:GanttChart"),
    ("SettingsComposer", "dcsset:SettingsComposer"),
    ("DataCompositionFilter", "dcsset:Filter"),
    ("DynamicList", "cfg:DynamicList"),
    ("ConstantsSet", "cfg:ConstantsSet"),
    ("ReportBuilder", "cfg:ReportBuilder"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata_object::{MdoType, PlatformValueType};

    fn parse(inner: &str) -> AttributeType {
        let xml = format!(
            r#"<mdclass:Catalog xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass"><type>{inner}</type></mdclass:Catalog>"#
        );
        let doc = roxmltree::Document::parse(&xml).unwrap();
        let node = doc.root_element().first_element_child().unwrap();
        parse_edt_type(node).unwrap()
    }

    #[test]
    fn primitive_tokens_carry_their_qualifiers() {
        assert_eq!(
            parse("<types>String</types><stringQualifiers><length>40</length></stringQualifiers>"),
            AttributeType::String { length: Some(40) }
        );
        assert_eq!(parse("<types>String</types>"), AttributeType::String { length: None });
        assert_eq!(
            parse("<types>Number</types><numberQualifiers><precision>5</precision><scale>2</scale></numberQualifiers>"),
            AttributeType::Number { precision: 5, scale: 2 }
        );
        assert_eq!(parse("<types>Boolean</types>"), AttributeType::Boolean);
        assert_eq!(
            parse("<types>Date</types><dateQualifiers><dateFractions>Date</dateFractions></dateQualifiers>"),
            AttributeType::Date
        );
        assert_eq!(
            parse("<types>Date</types><dateQualifiers><dateFractions>DateTime</dateFractions></dateQualifiers>"),
            AttributeType::DateTime
        );
        assert_eq!(parse("<types>UUID</types>"), AttributeType::Uuid);
        assert_eq!(parse("<types>ValueStorage</types>"), AttributeType::ValueStorage);
    }

    #[test]
    fn reference_tokens_resolve_like_their_designer_counterparts() {
        assert_eq!(
            parse("<types>CatalogRef.Сотрудники</types>"),
            AttributeType::Ref { mdo_type: MdoType::Catalog, name: "Сотрудники".into() }
        );
        assert_eq!(
            parse("<types>EnumRef.Статусы</types>"),
            AttributeType::Ref { mdo_type: MdoType::Enum, name: "Статусы".into() }
        );
        assert_eq!(
            parse("<types>DefinedType.Владелец</types>"),
            AttributeType::DefinedType { name: "Владелец".into() }
        );
        assert_eq!(
            parse("<types>Characteristic.Свойства</types>"),
            AttributeType::Ref {
                mdo_type: MdoType::ChartOfCharacteristicTypes,
                name: "Свойства".into()
            }
        );
        assert_eq!(parse("<types>AnyRef</types>"), AttributeType::AnyRef);
        assert_eq!(
            parse("<types>CatalogRef</types>"),
            AttributeType::AnyObjectRef { mdo_type: MdoType::Catalog }
        );
        // The same answer the configurator's `cfg:CatalogObject.Товары` gets.
        assert_eq!(
            parse("<types>CatalogObject.Товары</types>"),
            AttributeType::Ref { mdo_type: MdoType::Catalog, name: "Товары".into() }
        );
    }

    #[test]
    fn platform_tokens_and_composites() {
        assert_eq!(
            parse("<types>ValueList</types>"),
            AttributeType::Platform(PlatformValueType::ValueList)
        );
        assert_eq!(
            parse("<types>DynamicList</types>"),
            AttributeType::Platform(PlatformValueType::DynamicList)
        );
        assert_eq!(
            parse("<types>FormattedString</types>"),
            AttributeType::Platform(PlatformValueType::FormattedString)
        );
        match parse(
            "<types>String</types><types>CatalogRef.А</types><stringQualifiers><length>10</length></stringQualifiers>",
        ) {
            AttributeType::Composite { types } => assert_eq!(types.len(), 2),
            other => panic!("{other:?}"),
        }
        assert_eq!(parse(""), AttributeType::Unknown);
    }
}
