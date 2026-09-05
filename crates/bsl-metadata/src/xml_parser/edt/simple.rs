//! The one-descriptor kinds: defined types, event subscriptions, scheduled
//! jobs, subsystems, roles and the three services. Each is a handful of
//! camelCase properties of the root, read into the same model the configurator
//! reader builds.

use crate::defined_type::DefinedType;
use crate::error::{MetadataError, Result};
use crate::event_subscription::EventSubscription;
use crate::http_service::{
    HTTPService, HTTPServiceBuilder, HTTPServiceMethodBuilder, HTTPServiceURLTemplateBuilder,
};
use crate::integration_service::{
    IntegrationService, IntegrationServiceBuilder, IntegrationServiceChannelBuilder,
};
use crate::role::{Role, RoleData};
use crate::scheduled_job::ScheduledJob;
use crate::subsystem::Subsystem;
use crate::web_service::{
    WebService, WebServiceBuilder, WebServiceOperationBuilder, WebServiceParameter,
};

use super::super::helpers::{child_bool, child_text, find_child};
use super::super::subsystem::parse_mdo_ref;
use super::helpers::{child_texts, root_uuid};
use super::types::parse_edt_type;

pub(crate) fn parse_edt_defined_type(doc: &roxmltree::Document<'_>) -> Result<DefinedType> {
    let root = doc.root_element();
    let name = child_text(root, "name").unwrap_or("").to_string();
    let type_node = find_child(root, "type").ok_or_else(|| {
        MetadataError::InvalidFormat(format!("DefinedType '{name}' missing type element"))
    })?;
    Ok(DefinedType::builder()
        .uuid(root_uuid(root, "defined type")?)
        .name(name)
        .underlying_type(parse_edt_type(type_node)?)
        .build())
}

pub(crate) fn parse_edt_event_subscription(
    doc: &roxmltree::Document<'_>,
) -> Result<EventSubscription> {
    let root = doc.root_element();
    // The configurator joins `Type` and `TypeSet` entries with `;`; the EDT
    // `source/types` list is the same list.
    let source = find_child(root, "source").map(|s| child_texts(s, "types").join(";"));
    Ok(EventSubscription {
        uuid: root_uuid(root, "event subscription")?,
        name: child_text(root, "name").unwrap_or("").to_string(),
        comment: child_text(root, "comment").map(str::to_string),
        source: source.unwrap_or_default(),
        event: child_text(root, "event").unwrap_or("").to_string(),
        handler: child_text(root, "handler").unwrap_or("").to_string(),
    })
}

pub(crate) fn parse_edt_scheduled_job(doc: &roxmltree::Document<'_>) -> Result<ScheduledJob> {
    let root = doc.root_element();
    Ok(ScheduledJob {
        uuid: root_uuid(root, "scheduled job")?,
        name: child_text(root, "name").unwrap_or("").to_string(),
        method_name: child_text(root, "methodName").unwrap_or("").to_string(),
        predefined: child_bool(root, "predefined"),
        use_flag: child_bool(root, "use"),
    })
}

/// A subsystem: `content` members and `subsystems` children. A child is
/// written as its full path (`Subsystem.Родитель.Дочерняя`); the model keeps
/// the child's own name, as the configurator's `ChildObjects/Subsystem` does.
pub(crate) fn parse_edt_subsystem(doc: &roxmltree::Document<'_>) -> Result<Subsystem> {
    let root = doc.root_element();
    if root.tag_name().name() != "Subsystem" {
        return Err(MetadataError::InvalidFormat(format!(
            "expected a Subsystem element, found {}",
            root.tag_name().name()
        )));
    }
    let name = child_text(root, "name").unwrap_or("").to_string();
    let content = child_texts(root, "content").into_iter().filter_map(parse_mdo_ref).collect();
    let child_subsystems = child_texts(root, "subsystems")
        .into_iter()
        .filter_map(|path| path.rsplit('.').next())
        .filter(|child| !child.is_empty())
        .map(str::to_string)
        .collect();
    Ok(Subsystem::new(name).with_content(content).with_child_subsystems(child_subsystems))
}

pub(crate) fn parse_edt_role(doc: &roxmltree::Document<'_>) -> Result<Role> {
    let root = doc.root_element();
    Ok(Role::with_data(
        root_uuid(root, "role")?,
        child_text(root, "name").unwrap_or("").to_string(),
        RoleData::default(),
    ))
}

pub(crate) fn parse_edt_http_service(
    doc: &roxmltree::Document<'_>,
    name: &str,
) -> Result<HTTPService> {
    let root = doc.root_element();
    let mut builder = HTTPServiceBuilder::new()
        .name(child_text(root, "name").unwrap_or(""))
        .root_url(child_text(root, "rootURL").unwrap_or(""))
        .uri(format!("HTTPServices/{name}/Module.bsl"));
    for template_node in
        root.children().filter(|n| n.is_element() && n.tag_name().name() == "urlTemplates")
    {
        let mut template = HTTPServiceURLTemplateBuilder::new()
            .name(child_text(template_node, "name").unwrap_or(""))
            .template(child_text(template_node, "template").unwrap_or(""));
        for method_node in
            template_node.children().filter(|n| n.is_element() && n.tag_name().name() == "methods")
        {
            template = template.add_method(
                HTTPServiceMethodBuilder::new()
                    .name(child_text(method_node, "name").unwrap_or(""))
                    .http_method(child_text(method_node, "httpMethod").unwrap_or(""))
                    .handler(child_text(method_node, "handler").unwrap_or(""))
                    .build(),
            );
        }
        builder = builder.add_url_template(template.build());
    }
    Ok(builder.build())
}

pub(crate) fn parse_edt_web_service(
    doc: &roxmltree::Document<'_>,
    name: &str,
) -> Result<WebService> {
    let root = doc.root_element();
    let mut builder = WebServiceBuilder::new()
        .name(child_text(root, "name").unwrap_or(""))
        .namespace(child_text(root, "namespace").unwrap_or(""))
        .uri(format!("WebServices/{name}/Module.bsl"));
    for operation_node in
        root.children().filter(|n| n.is_element() && n.tag_name().name() == "operations")
    {
        let mut operation = WebServiceOperationBuilder::new()
            .name(child_text(operation_node, "name").unwrap_or(""))
            .procedure_name(child_text(operation_node, "procedureName").unwrap_or(""));
        for parameter_node in operation_node
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "parameters")
        {
            operation = operation.add_parameter(WebServiceParameter::new(
                child_text(parameter_node, "name").unwrap_or(""),
            ));
        }
        builder = builder.add_operation(operation.build());
    }
    Ok(builder.build())
}

pub(crate) fn parse_edt_integration_service(
    doc: &roxmltree::Document<'_>,
    name: &str,
) -> Result<IntegrationService> {
    let root = doc.root_element();
    let mut builder =
        IntegrationServiceBuilder::new().name(child_text(root, "name").unwrap_or(name));
    for channel_node in root
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "integrationServiceChannels")
    {
        builder = builder.add_channel(
            IntegrationServiceChannelBuilder::new()
                .name(child_text(channel_node, "name").unwrap_or(""))
                .receive_message_processing(
                    child_text(channel_node, "receiveMessageProcessing").unwrap_or(""),
                )
                .build(),
        );
    }
    Ok(builder.build())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata_object::{AttributeType, MdoType};

    #[test]
    fn edt_defined_type_subscription_job_subsystem_role_and_services() {
        let dt = roxmltree::Document::parse(r#"<mdclass:DefinedType xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="b0ca1b85-e1de-4b7c-8261-7f9e8a6a72eb"><name>Тип1</name><type><types>CatalogRef.А</types><types>CatalogRef.Б</types></type></mdclass:DefinedType>"#).unwrap();
        let dt = parse_edt_defined_type(&dt).unwrap();
        assert_eq!(dt.name(), "Тип1");
        assert!(matches!(dt.underlying_type(), AttributeType::Composite { .. }));

        let es = roxmltree::Document::parse(r#"<mdclass:EventSubscription xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="4e36184e-d818-4f38-9257-6d91d9cc85ae"><name>Подписка1</name><source><types>CatalogObject.А</types><types>CatalogObject.Б</types></source><event>BeforeWrite</event><handler>CommonModule.М.ПередЗаписью</handler></mdclass:EventSubscription>"#).unwrap();
        let es = parse_edt_event_subscription(&es).unwrap();
        assert_eq!(es.handler_string(), "CommonModule.М.ПередЗаписью");
        assert_eq!(es.event(), "BeforeWrite");
        assert_eq!(es.source(), "CatalogObject.А;CatalogObject.Б");

        let job = roxmltree::Document::parse(r#"<mdclass:ScheduledJob xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="2cce2723-f2f0-41ff-b2b5-b661ac1ccbbb"><name>Задание1</name><methodName>CommonModule.М.Выполнить</methodName><use>true</use><predefined>true</predefined></mdclass:ScheduledJob>"#).unwrap();
        let job = parse_edt_scheduled_job(&job).unwrap();
        assert_eq!(job.method_name(), "CommonModule.М.Выполнить");
        assert!(job.is_predefined() && job.is_enabled());

        let sub = roxmltree::Document::parse(r#"<mdclass:Subsystem xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="e1a5b582-c7f3-432d-9006-7094b6415917"><name>Подсистема1</name><content>Catalog.А</content><content>CommonCommand.К</content><subsystems>Subsystem.Подсистема1.Подсистема2</subsystems></mdclass:Subsystem>"#).unwrap();
        let sub = parse_edt_subsystem(&sub).unwrap();
        assert_eq!(sub.content(), &[(MdoType::Catalog, "А".to_string())]);
        assert_eq!(sub.child_subsystems(), &["Подсистема2".to_string()]);
        let not_a_subsystem = roxmltree::Document::parse(r#"<mdclass:Catalog xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="e1a5b582-c7f3-432d-9006-7094b6415917"><name>Н</name></mdclass:Catalog>"#).unwrap();
        assert!(parse_edt_subsystem(&not_a_subsystem).is_err());

        let role = roxmltree::Document::parse(r#"<mdclass:Role xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="68b0d093-d1a6-4425-bef8-91e2db3e456c"><name>Роль1</name></mdclass:Role>"#).unwrap();
        assert_eq!(parse_edt_role(&role).unwrap().name(), "Роль1");

        let http = roxmltree::Document::parse(r#"<mdclass:HTTPService xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="c6dd7b6f-b459-404b-b743-a481fa6f6fac"><name>HTTPСервис1</name><rootURL>api</rootURL><urlTemplates uuid="5111ecf5-2782-4bd0-a62f-baf32caef36f"><name>Шаблон</name><template>/{id}</template><methods uuid="2baef4f6-928a-491a-b983-3a3d5c0664fa"><name>GET</name><httpMethod>GET</httpMethod><handler>ШаблонGET</handler></methods></urlTemplates></mdclass:HTTPService>"#).unwrap();
        let http = parse_edt_http_service(&http, "HTTPСервис1").unwrap();
        assert_eq!(http.root_url(), "api");
        assert_eq!(http.uri(), Some("HTTPServices/HTTPСервис1/Module.bsl"));
        assert_eq!(http.url_templates()[0].template(), "/{id}");
        assert_eq!(http.url_templates()[0].methods()[0].handler(), "ШаблонGET");
        assert_eq!(http.url_templates()[0].methods()[0].http_method(), "GET");

        let web = roxmltree::Document::parse(r#"<mdclass:WebService xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="c6dd7b6f-b459-404b-b743-a481fa6f6fad"><name>WebСервис1</name><namespace>http://x</namespace><operations uuid="c6dd7b6f-b459-404b-b743-a481fa6f6fae"><name>Оп</name><procedureName>ОпВыполнить</procedureName><parameters uuid="c6dd7b6f-b459-404b-b743-a481fa6f6faf"><name>П</name></parameters></operations></mdclass:WebService>"#).unwrap();
        let web = parse_edt_web_service(&web, "WebСервис1").unwrap();
        assert_eq!(web.namespace(), "http://x");
        assert_eq!(web.uri(), Some("WebServices/WebСервис1/Module.bsl"));
        assert_eq!(web.operations()[0].procedure_name(), "ОпВыполнить");
        assert_eq!(web.operations()[0].parameters()[0].name(), "П");

        let is = roxmltree::Document::parse(r#"<mdclass:IntegrationService xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="c6dd7b6f-b459-404b-b743-a481fa6f6fb0"><name>Сервис1</name><integrationServiceChannels uuid="c6dd7b6f-b459-404b-b743-a481fa6f6fb1"><name>Канал</name><receiveMessageProcessing>ОбработатьСообщение</receiveMessageProcessing></integrationServiceChannels></mdclass:IntegrationService>"#).unwrap();
        let is = parse_edt_integration_service(&is, "Сервис1").unwrap();
        assert_eq!(is.channels()[0].receive_message_processing(), "ОбработатьСообщение");
    }

    #[test]
    fn public_parsers_dispatch_on_the_namespace() {
        let dt = crate::xml_parser::parse_defined_type_xml(r#"<mdclass:DefinedType xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="b0ca1b85-e1de-4b7c-8261-7f9e8a6a72eb"><name>Тип1</name><type><types>String</types></type></mdclass:DefinedType>"#).unwrap();
        assert_eq!(dt.name(), "Тип1");
        let es = crate::xml_parser::parse_event_subscription_xml(r#"<mdclass:EventSubscription xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="4e36184e-d818-4f38-9257-6d91d9cc85ae"><name>П</name><event>OnWrite</event><handler>CommonModule.М.П</handler></mdclass:EventSubscription>"#).unwrap();
        assert_eq!(es.event(), "OnWrite");
        let job = crate::xml_parser::parse_scheduled_job_xml(r#"<mdclass:ScheduledJob xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="2cce2723-f2f0-41ff-b2b5-b661ac1ccbbb"><name>З</name><methodName>CommonModule.М.В</methodName></mdclass:ScheduledJob>"#).unwrap();
        assert_eq!(job.method_name(), "CommonModule.М.В");
        let sub = crate::xml_parser::parse_subsystem_xml(r#"<mdclass:Subsystem xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="e1a5b582-c7f3-432d-9006-7094b6415917"><name>С</name></mdclass:Subsystem>"#).unwrap();
        assert_eq!(sub.name(), "С");
        let role = crate::xml_parser::parse_role_xml(r#"<mdclass:Role xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="68b0d093-d1a6-4425-bef8-91e2db3e456c"><name>Р</name></mdclass:Role>"#).unwrap();
        assert_eq!(role.name(), "Р");
        let http = crate::xml_parser::parse_http_service_xml(r#"<mdclass:HTTPService xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="c6dd7b6f-b459-404b-b743-a481fa6f6fac"><name>H</name><rootURL>h</rootURL></mdclass:HTTPService>"#, "H").unwrap();
        assert_eq!(http.root_url(), "h");
        let web = crate::xml_parser::parse_web_service_xml(r#"<mdclass:WebService xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="c6dd7b6f-b459-404b-b743-a481fa6f6fad"><name>W</name><namespace>n</namespace></mdclass:WebService>"#, "W").unwrap();
        assert_eq!(web.namespace(), "n");
        let is = crate::xml_parser::parse_integration_service_xml(r#"<mdclass:IntegrationService xmlns:mdclass="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="c6dd7b6f-b459-404b-b743-a481fa6f6fb0"><name>I</name></mdclass:IntegrationService>"#, "I").unwrap();
        assert_eq!(is.name(), "I");
    }
}
