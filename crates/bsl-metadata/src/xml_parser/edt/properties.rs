use super::super::helpers::{child_bool, child_text, child_u32};
use super::super::standard_attributes::MdoProperties;
use super::helpers::child_texts;

/// The same properties the configurator keeps under `Properties`, read from
/// the camelCase children of an EDT root. Feeds the standard-attribute tables
/// unchanged, so `Код`, `Владелец`, `Период` and friends come out identical.
pub(crate) fn edt_properties(root: roxmltree::Node<'_, '_>) -> MdoProperties {
    MdoProperties {
        name: child_text(root, "name").unwrap_or("").to_string(),
        code_length: child_u32(root, "codeLength"),
        code_type: child_text(root, "codeType").map(str::to_string),
        number_length: child_u32(root, "numberLength"),
        number_type: child_text(root, "numberType").map(str::to_string),
        description_length: child_u32(root, "descriptionLength"),
        hierarchical: child_bool(root, "hierarchical"),
        owners: child_texts(root, "owners").into_iter().map(str::to_string).collect(),
        periodicity: child_text(root, "informationRegisterPeriodicity").map(str::to_string),
        check_unique: child_bool(root, "checkUnique"),
        code_series: child_text(root, "codeSeries").map(str::to_string),
        dependence_on_calculation_types: child_text(root, "dependenceOnCalculationTypes")
            .map(str::to_string),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camel_case_properties_land_in_the_same_struct() {
        let doc = roxmltree::Document::parse(
            r#"<mdclass:Catalog xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="a"><name>Товары</name><hierarchical>true</hierarchical><owners>Catalog.А</owners><owners>Catalog.Б</owners><codeLength>9</codeLength><descriptionLength>25</descriptionLength><codeType>Number</codeType><codeSeries>WholeCatalog</codeSeries><checkUnique>true</checkUnique></mdclass:Catalog>"#,
        )
        .unwrap();
        let p = edt_properties(doc.root_element());
        assert_eq!(p.name, "Товары");
        assert!(p.hierarchical && p.check_unique);
        assert_eq!(p.owners, vec!["Catalog.А", "Catalog.Б"]);
        assert_eq!(p.code_length, Some(9));
        assert_eq!(p.description_length, Some(25));
        assert_eq!(p.code_type.as_deref(), Some("Number"));
        assert_eq!(p.code_series.as_deref(), Some("WholeCatalog"));
        assert_eq!(p.number_length, None);
        assert_eq!(p.periodicity, None);
        assert!(!p.depends_on_calculation_types());
    }
}
