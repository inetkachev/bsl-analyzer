# Поддержка формата 1C:EDT в `bsl-metadata`: дизайн

Дата: 2026-09-06. Ветка: `feature/edt-metadata`. Цель — pull request в апстрим
`itrous/bsl-analyzer`.

## 1. Цель

Анализатор читает метаданные конфигурации только из выгрузки Конфигуратора
(Designer XML). На проекте в формате 1C:EDT загрузчик молча возвращает ноль
объектов, и все правила, которым нужны метаданные, дают ложные срабатывания.

После этой работы:

- `bsl-analyzer analyze -s <корень EDT-проекта>` загружает те же метаданные,
  что загрузил бы с выгрузки Конфигуратора той же конфигурации;
- формат определяется автоматически по маркеру корня, без флагов;
- заимствованные объекты расширения (`objectBelonging=Adopted`) отличимы от
  собственных в модели данных;
- выгрузка Конфигуратора обрабатывается ровно как раньше: это регрессионное
  условие, а не пожелание.

## 2. Границы

Входит:

- крейт `bsl-metadata`: обнаружение раскладки EDT и разбор `.mdo`, `Form.form`,
  `Rights.rights`;
- крейт `bsl-conventions`: словарь конвенционных имён EDT;
- крейт `project-model`: маркер корня, вид конфигурации, роли файлов;
- крейты `ide-db` и `hir-def`: формы путей модулей EDT;
- одна точка в CLI `bsl-analyzer` (проверка `--configuration-root` и
  `--extension`), сведённая к вызову хелпера из `project-model`;
- фикстуры `crates/bsl-metadata/fixtures/edt/` и `fixtures/edt_extension/`;
- документация: `docs/configuration/PROJECT_CONFIGURATION.md`, `CHANGELOG.md`.

Не входит:

- крейты Tier B (`parser`, `lexer`, `sdbl-hir`, `ide-diagnostics`) не
  редактируются;
- отладчик (`bsl-debug`) и его индекс модулей остаются на раскладке
  Конфигуратора;
- инструмент MCP `metadata forms` и подписи путей в `bsl-search::context`
  остаются как есть;
- зависимости на 1C:EDT, платформу 1С или Java не добавляются: разбор чисто
  файловый;
- разбор Designer не переписывается; допустимо только чтение одного нового
  свойства (`ObjectBelonging`), нужного для симметрии модели.

## 3. Установленные факты

Всё ниже проверено на реальных проектах на этой машине: ЗУП КОРП 3.1.38.18
(`EDTWorkspace_CORP/git/zup3-edt/src/project/src`, 12 092 `.mdo`, 12 433
`.bsl`, 4 604 `Form.form`), расширение `WA_ЛК_Сотрудника`
(`EDTWorkspace/git/empldocs-extension-edt/src/project/src`, 1 352 `.mdo`),
расширение `ИсправлениеОбезличиванияПД` (7 `.mdo`, 5 `.bsl`, без форм).

### 3.1 Раскладка EDT

Корень исходников EDT — каталог `src` проекта. Каждый объект — каталог
`<Коллекция>/<Имя>/`, описание — `<Имя>.mdo` внутри него. Служебного уровня
`Ext/` нет. Сравнение с выгрузкой Конфигуратора:

| Что | Designer | EDT |
|---|---|---|
| маркер корня | `Configuration.xml` | `Configuration/Configuration.mdo` |
| корневые модули | `Ext/SessionModule.bsl`, `Ext/ManagedApplicationModule.bsl`, … | `Configuration/SessionModule.bsl`, `Configuration/ManagedApplicationModule.bsl`, `Configuration/OrdinaryApplicationModule.bsl`, `Configuration/ExternalConnectionModule.bsl` |
| описание объекта | `<Коллекция>/<Имя>.xml` рядом с каталогом | `<Коллекция>/<Имя>/<Имя>.mdo` |
| модули объекта | `<Имя>/Ext/ObjectModule.bsl`, `ManagerModule.bsl`, `RecordSetModule.bsl`, `ValueManagerModule.bsl` | те же имена прямо в `<Имя>/` |
| общий модуль | `CommonModules/<Имя>.xml` + `<Имя>/Ext/Module.bsl` | `CommonModules/<Имя>/<Имя>.mdo` + `<Имя>/Module.bsl` |
| предопределённые | `<Имя>/Ext/Predefined.xml` | узел `<predefined>` внутри `.mdo` |
| форма объекта | `<Имя>/Forms/<Форма>.xml` + `Forms/<Форма>/Ext/Form.xml` + `Forms/<Форма>/Ext/Form/Module.bsl` | `<Имя>/Forms/<Форма>/Form.form` + `Forms/<Форма>/Module.bsl`; имя и тип формы — в узле `<forms>` `.mdo` владельца |
| общая форма | `CommonForms/<Форма>.xml` + `<Форма>/Ext/Form.xml` + `<Форма>/Ext/Form/Module.bsl` | `CommonForms/<Форма>/<Форма>.mdo` + `Form.form` + `Module.bsl` |
| команда объекта | `<Имя>/Commands/<Команда>/Ext/CommandModule.bsl` | `<Имя>/Commands/<Команда>/CommandModule.bsl` |
| общая команда | `CommonCommands/<Имя>.xml` + `<Имя>/Ext/CommandModule.bsl` | `CommonCommands/<Имя>/<Имя>.mdo` + `CommandModule.bsl` |
| роль | `Roles/<Имя>.xml` + `<Имя>/Ext/Rights.xml` | `Roles/<Имя>/<Имя>.mdo` + `Rights.rights` |
| подсистема | `Subsystems/<Имя>.xml` + `<Имя>/Subsystems/<Дочерняя>.xml` | `Subsystems/<Имя>/<Имя>.mdo` + `<Имя>/Subsystems/<Дочерняя>/<Дочерняя>.mdo` |
| перечисление, константа, определяемый тип, подписка, регламентное задание | одиночный `<Коллекция>/<Имя>.xml` | каталог `<Коллекция>/<Имя>/<Имя>.mdo` (константа — ещё `ValueManagerModule.bsl`) |
| HTTP-, web-, сервис интеграции | `<Коллекция>/<Имя>.xml` + `<Имя>/Ext/Module.bsl` | `<Коллекция>/<Имя>/<Имя>.mdo` + `Module.bsl` |
| защищённый модуль | `Ext/Module.bin` без `Module.bsl` | не бывает: EDT хранит только исходники |

Коллекции без `MdoType` в анализаторе (журналы документов, хранилища настроек,
критерии отбора, нумераторы, последовательности, перерасчёты) устроены так же:
каталог, `.mdo`, модули без `Ext/`.

Проект EDT целиком (`.project`, `DT-INF/PROJECT.PMF`, `src/`) может лежать в
корне репозитория или глубже (`src/project/` у `empldocs-extension-edt`).

### 3.2 Содержимое `.mdo`

Пространство имён корня `http://g5.1c.ru/v8/dt/metadata/mdclass`, корневой
элемент `mdclass:<Вид>` (`mdclass:Catalog`, `mdclass:CommonModule`, …),
атрибут `uuid`. Свойства — прямые дочерние элементы в camelCase без обёртки
`Properties`; повторяющиеся свойства повторяют элемент
(`<owners>Catalog.ФизическиеЛица</owners>` дважды для двух владельцев).

Соответствия, которые нужны загрузчику:

| Designer (`Properties/…` или `ChildObjects/…`) | EDT |
|---|---|
| `Name` | `name` |
| `Comment` | `comment` |
| `ObjectBelonging` | `objectBelonging` (`Adopted` только у заимствованных; у собственных элемента нет) |
| `CodeLength`, `CodeType`, `DescriptionLength`, `Hierarchical`, `CheckUnique`, `CodeSeries` | `codeLength`, `codeType`, `descriptionLength`, `hierarchical`, `checkUnique`, `codeSeries` |
| `NumberLength`, `NumberType` | `numberLength`, `numberType` |
| `Owners/Item` | `owners` (повторяется) |
| `InformationRegisterPeriodicity`, `RegisterType` | `informationRegisterPeriodicity`, `registerType` |
| `DependenceOnCalculationTypes` | `dependenceOnCalculationTypes` |
| `RegisterRecords/Item` | `registerRecords` (повторяется) |
| `ChildObjects/Attribute` | `attributes uuid=…` (повторяется) |
| `ChildObjects/TabularSection` + вложенные `Attribute` | `tabularSections uuid=…` с вложенными `attributes` |
| `ChildObjects/Dimension`, `Resource` | `dimensions`, `resources` |
| `ChildObjects/EnumValue` | `enumValues uuid=…` |
| `ChildObjects/Form` | `forms uuid=…` (`name`, при необходимости `formType`) |
| `ChildObjects/Subsystem` | `subsystems` (`Subsystem.Родитель.Дочерняя`) |
| `Content/Item` подсистемы | `content` (`Catalog.Имя`) |
| `Ext/Predefined.xml` | `predefined/items id=…` с `name`, вложенные `items` |
| `Properties/Type` | `type` |
| `Source/Type`, `Event`, `Handler` подписки | `source/types`, `event`, `handler` |
| `MethodName`, `Use`, `Predefined` задания | `methodName`, `use`, `predefined` |
| `Global`, `Server`, `ClientManagedApplication`, `ClientOrdinaryApplication`, `ExternalConnection`, `ServerCall`, `Privileged`, `ReturnValuesReuse` общего модуля | те же имена в camelCase; `false` не записывается, элемент просто отсутствует |
| `RootURL`, `URLTemplate/Template`, `Method/HTTPMethod`, `Handler` | `rootURL`, `urlTemplates/template`, `methods/httpMethod`, `handler` |
| `Namespace`, `Operation/ProcedureName`, `Parameter` | `namespace`, `operations/procedureName`, `parameters` |
| `IntegrationServiceChannel/ReceiveMessageProcessing` | `integrationServiceChannels/receiveMessageProcessing` |

Типы. Designer: `<Type><v8:Type>cfg:CatalogRef.Имя</v8:Type><v8:StringQualifiers>…`.
EDT: `<type><types>CatalogRef.Имя</types><stringQualifiers><length>40</length></stringQualifiers>`.
Токены EDT — те же имена без префиксов пространств: `String`, `Number`,
`Boolean`, `Date`, `UUID`, `ValueStorage`, `AnyRef`, `CatalogRef.X`,
`DocumentRef.X`, `EnumRef.X`, `DefinedType.X`, `Characteristic.X`,
`CatalogObject.X`, платформенные `ValueList`, `ValueTable`, `FixedArray`, …
Квалификаторы: `stringQualifiers/length`, `numberQualifiers/precision|scale`,
`dateQualifiers/dateFractions` (`Date`, `DateTime`, `Time`). Составной тип —
несколько `<types>`.

Расширение конфигурации. Корень `Configuration.mdo` расширения несёт
`objectBelonging=Adopted`, `configurationExtensionPurpose`, `namePrefix`.
Заимствованный объект несёт атрибут `extendedConfigurationObject` (uuid
объекта базовой конфигурации), элемент `objectBelonging=Adopted` и узел
`<extension xsi:type="mdclassExtension:<Вид>Extension">`, в котором
перечислены контролируемые свойства со значениями `Checked`, `Unchecked` или
`Extended`. Собственные объекты расширения этих узлов не имеют.

**Поправка к заданию.** Значения `Checked`/`Unchecked` — не булевы значения
свойств. Логические свойства в EDT записываются как `true`/`false`
(`<server>true</server>`), а `Checked`/`Unchecked`/`Extended` встречаются
только внутри узла `<extension>` и означают «расширение контролирует это
свойство». Загрузчик читает булевы значения как в Designer; узел
`<extension>` целиком игнорируется, кроме факта заимствования.

Заимствованный объект в `.mdo` расширения описывает только то, что расширение
добавило: заимствованный справочник без добавленных реквизитов — это 15 строк
без единого `attributes`. Слияние с базой выполняет уже существующий
`Configuration::merge_extension_overlay`, семантика та же, что для выгрузки
Designer-расширений.

### 3.3 Формы EDT

`Form.form`: пространство имён `http://g5.1c.ru/v8/dt/form`, корень
`form:Form`. Элементы — `<items xsi:type="form:FormGroup|form:FormField|form:Button|form:Table|form:Decoration">`
с `<name>`, `<id>`, `<dataPath><segments>…</segments></dataPath>`, вложенными
`<items>` и обработчиками `<handlers><event>OnChange</event><name>Имя</name></handlers>`;
у полей и групп подтип в `<type>` (`InputField`, `UsualGroup`, `Label`, …).
Реквизиты — `<attributes><name>…</name><valueType><types>…</types></valueType><main>true</main>`
и вложенные `<columns>` у таблиц значений. Команды — `<formCommands><name>…</name><action><handler><name>…</name></handler></action>`.
Обработчики формы — `<handlers>` на верхнем уровне. Имя, uuid и тип формы —
в узле `<forms>` `.mdo` владельца (или в `<Имя>.mdo` общей формы); тип
отсутствует у управляемых форм.

### 3.4 Где формат зашит в коде

Раскладка Конфигуратора предполагается в следующих местах; каждое из них
затрагивает дизайн:

| Место | Что предполагает |
|---|---|
| `bsl-metadata::loader` | `<Имя>.xml` рядом с каталогом, `Ext/<файл>`, `Ext/Predefined.xml`, `Ext/Rights.xml`, одиночные `.xml` у перечислений и констант |
| `bsl-metadata::xml_parser::*` | корень `MetaDataObject`, обёртка `Properties`, `ChildObjects`, префиксованные токены типов |
| `bsl-metadata::xml_parser::form::parse_form_from_bsl_path` | `Forms/<Ф>/Ext/Form/Module.bsl`, `Ext/Form.xml`, `Forms/<Ф>.xml` |
| `bsl-conventions` | словарь: нет `.mdo`, `Form.form`, `Rights.rights`, каталога `Configuration` |
| `project-model::discover_source_path`, `search_configuration`, `configuration_kind`, `validate_extension`, `classify_external_root` | маркер `Configuration.xml`, признак расширения `<ConfigurationExtensionPurpose` |
| `project-model::file_role` | в VFS попадают только `.bsl` и `.xml`; тело общего модуля узнаётся по `Ext/Module.bsl` |
| `ide-db::metadata::get_module_type_from_uri` | корневые модули по `Ext/`, формы по `Forms/<Ф>/Ext/Form/Module.bsl`, коллекции без `MdoType` — только с `Ext/` |
| `hir-def::module_index::parse_form_module_path`, `module_path_segment_modes`, `configs::ApplicationModuleKind::relative_path` | те же формы путей; путь корневого модуля строится как `Ext/<файл>` |
| `bsl-analyzer` CLI `source_set.rs`, `check_config.rs` | `--configuration-root` и `--extension` требуют `Configuration.xml` |
| `bsl-conventions::path_ends_with_ext_child` | «корневой модуль» = `…/Ext/<файл>`; через него пять диагностик Tier B узнают модуль сеанса и модуль управляемого приложения |

Обе ветки загрузки — полная (`load_from_directory`, батч и LSP после
загрузки) и пообъектная (`discover_*_structure` + `parse_*_from_text`, LSP и
MCP) — идут через один и тот же крейт, поэтому правка раскладки и разбора в
`bsl-metadata` покрывает обе.

### 3.5 Замеры «до» (бинарник v0.2.77)

| Проект | Общих модулей | Объектов |
|---|---|---|
| `apard_xml` (Designer) | 40 | 871 |
| `empldocs-extension-edt` (EDT) | 0 | 0 |
| `ИсправлениеОбезличиванияПД` (EDT) | 0 | 0 |

## 4. Архитектура

Формат — это два независимых свойства, и определяются они в двух местах:

1. **Раскладка** — свойство корня. `SourceFormat::detect(root, tree)` отвечает
   `Edt`, если под корнем есть `Configuration/Configuration.mdo` (регистр
   конвенционных сегментов не значим), иначе `Designer`. Отсутствие обоих
   маркеров означает `Designer`: так ведут себя сегодняшние тесты на временных
   каталогах без `Configuration.xml`, и это поведение сохраняется. Детектор
   вызывается один раз на корень в `load_from_directory` и в каждой
   `discover_*_structure`; проба — один `stat`.
2. **Содержимое** — свойство документа. Каждый публичный `parse_<вид>_xml(text)`
   разбирает XML, смотрит на пространство имён корневого элемента и передаёт
   документ либо существующему разбору Designer (код не меняется, только
   переезжает в приватную функцию), либо новому разбору EDT. Публичные
   сигнатуры, включая `parse_metadata_object_from_texts`, не меняются, поэтому
   пообъектные запросы Salsa в `ide-db` не знают о формате вовсе.

Оба разбора складывают результат в одну и ту же модель (`MetadataObject`,
`CommonModule`, `Register`, `Form`, …). Ни один потребитель выше
`bsl-metadata` не видит «формат» как понятие, кроме `project-model`
(маркер корня) и классификаторов путей модулей (`ide-db`, `hir-def`), которые
получают вторую форму пути.

Почему не единый «нормализующий» слой, который переводил бы `.mdo` в дерево
Designer XML: он удвоил бы объём XML в памяти на конфигурации из 12 тысяч
объектов и потребовал бы описать все правила Designer наоборот. Почему не
отдельный крейт `bsl-metadata-edt`: разбор EDT использует те же таблицы
стандартных реквизитов, тот же `MdoProperties` и ту же модель; граница крейта
прошла бы через середину одного алгоритма.

## 5. Компоненты

### 5.1 `bsl-conventions`

В словарь `ConventionalName` добавляются: `ConfigurationDir` («Configuration»,
каталог), `ConfigurationMdo` («Configuration.mdo»), `FormForm` («Form.form»),
`RightsRights` («Rights.rights»). Константа `MDO_EXTENSION = "mdo"`.
Снимок состава словаря (`the_dictionary_composition_is_pinned`) обновляется;
из списка «не конвенционных» имён убирается `Configuration`.

`path_ends_with_ext_child(path, name)` начинает принимать вторую форму
служебного уровня: родитель `Configuration` наравне с `Ext`. Функция
переименовывается в `path_is_root_module` с прежней сигнатурой; старое имя
остаётся как обёртка, потому что пять диагностик Tier B вызывают его по
имени и не редактируются. Так модуль сеанса и модуль управляемого приложения
EDT-проекта распознаются этими диагностиками без правки Tier B. Объект не
может называться `Configuration` на позиции служебного уровня: под
`<Коллекция>/Configuration/` лежал бы `ObjectModule.bsl`, а не
`SessionModule.bsl`, так что двусмысленности нет.

### 5.2 `bsl-metadata`

**`SourceFormat`** живёт в `bsl-conventions/src/layout.rs`, рядом с `DirTree`
и пробами, потому что это правило раскладки, а не разбора:
`pub enum SourceFormat { Designer, Edt }`, `SourceFormat::detect(root, &dyn DirTree)`,
`SourceFormat::of_directory(root)` для реальной ФС,
`edt_configuration_mdo_in(root, tree)`. `bsl-metadata` реэкспортирует тип из
`lib.rs`; `project-model` берёт его оттуда же.

**`loader.rs`.** Пробы раскладки получают параметр формата. Для EDT одно
правило на все коллекции: объект — это каталог `<Имя>/` с файлом `<Имя>.mdo`
внутри (стем сравнивается точно, расширение без учёта регистра — как у
`probe_sibling_xml`). Модули — `find_child_ci_in(dir, <файл>)` без `Ext/`;
`probe_ext_child` получает EDT-ветку. Предопределённые — `None` (внутри
`.mdo`). Права роли — `Rights.rights`. Одиночные `.xml`-объекты (перечисления,
константы, подписки, задания, определяемые типы) в EDT — каталоги. Подсистемы
— рекурсия по `<Имя>/Subsystems/`. Сервисы — каталог с `.mdo` и `Module.bsl`.
`URI` общего модуля и сервисов строится из реального пути
(`CommonModules/<Имя>/Module.bsl`) — как сейчас из реального `Ext/…`.
Обе ветки обнаружения (`load_*_parallel` по `RealFs` и `discover_*_structure`
по `DirTree`) вызывают одни и те же пробы, поэтому тест
`both_tree_sources_discover_the_same_structure` расширяется на фикстуру EDT.

**`xml_parser/edt/`** (новый модуль): `helpers.rs` (детект пространства
имён, `child_texts` для повторяющихся элементов), `types.rs` (токены и
квалификаторы EDT → `AttributeType`, с той же таблицей ссылочных типов;
токен без префикса нормализуется к Designer-ключу `cfg:<токен>` и уходит в
существующие таблицы, так что список видов ссылок не дублируется),
`properties.rs` (`MdoProperties::from_edt_root` — те же поля, что у
`from_node`, из camelCase-элементов; дальше работают существующие
`add_*_standard_attributes`), `metadata_object.rs` (семейство справочника:
реквизиты, табличные части, регистраторы документа, предопределённые,
значения перечисления, тип константы, флаги плана счетов и зависимость плана
видов расчёта), `register.rs`, `common_module.rs`, `defined_type.rs`,
`event_subscription.rs` (`source/types` склеиваются через `;`, как Designer
склеивает `Type` и `TypeSet`), `scheduled_job.rs`, `subsystem.rs`, `role.rs`
(имя роли; права — существующий `parse_rights_xml`, схема `Rights.rights`
совпадает со схемой `Rights.xml`: корень `Rights` в пространстве
`http://v8.1c.ru/8.2/roles`), `services.rs`, `form.rs`.

Существующие `parse_*_xml` в `xml_parser/*.rs` становятся диспетчерами по
`DocumentFormat::of(&doc)`. Разбор Designer не меняется, кроме одного
добавления: `Properties/ObjectBelonging` читается в поле модели.

**Модель.** `MetadataObject` и `Register` получают поле
`object_belonging: ObjectBelonging` (serde `default`, значение `Own`);
`CommonModule` его уже имеет, но не заполняет. `apply_extension_overlay`
поле не переносит: у слитого объекта базы принадлежность остаётся `Own`, а
у объекта в конфигурации расширения (до слияния) — `Adopted`. Это и есть
различимость из критерия 4: правило, читающее конфигурацию расширения,
видит `Adopted`, правило над слитой конфигурацией — состав базы.

**Формы.** `parse_form_from_bsl_path(bsl_path)` определяет раскладку по
соседям модуля: если рядом лежит `Form.form` — это форма EDT, имя формы —
имя каталога, владелец — `.mdo` на три уровня выше (`<Имя>/<Имя>.mdo`) или
`CommonForms/<Форма>/<Форма>.mdo`; из узла `<forms>` с тем же `name` берутся
`uuid` и `formType`. Разбор `Form.form` даёт ту же модель `Form`:
элементы с родителем и видом (`FormGroup`→по `<type>`: `UsualGroup`, `Pages`,
`Page`, `CommandBar`, `ButtonGroup`; `FormField`→`Field`; `Button`;
`Table`; `Decoration`; `extendedTooltip`, `contextMenu`, `autoCommandBar`
→`Addition`), `dataPath` из `segments` через точку, обработчики событий
элементов и формы, обработчики команд из `formCommands`, реквизиты с типом,
главный реквизит по `<main>true</main>`, колонки табличных реквизитов.

### 5.3 `project-model`

`configuration_xml_in(dir)` заменяется на `configuration_marker_in(dir) ->
Option<ConfigurationMarker>` с полями `path` и `format`; функция публичная,
её же вызывает CLI. `configuration_kind(dir)` читает маркер и ищет
`<ConfigurationExtensionPurpose` (Designer) или `<configurationExtensionPurpose`
(EDT); для `.mdo` порог раннего выхода — `</mdclass:Configuration>`, верхняя
граница чтения та же. Все шесть мест, где сегодня проверяется
`Configuration.xml` (`discover_source_path`, `search_configuration_recursive`,
`validate_extension`, структурные записи расширений в `extension_topology`,
`classify_external_root`, сообщения об ошибках), переходят на маркер. Поиск
в глубину два уровня и конвенционные каталоги `src/cf`, `Configuration` не
меняются; для EDT это значит, что `-s <корень проекта EDT>` находит `src`, а
`-s <корень src>` — сам себя.

`file_role.rs`: `METADATA_WATCHED_EXTENSIONS` дополняется `mdo`, `form`,
`rights` (файлы попадают в VFS и в наблюдение LSP наравне с `.xml`);
`substrate_listed_family` принимает `<Семейство>/<Имя>/Module.bsl` без `Ext/`.

### 5.4 `ide-db` и `hir-def`

`get_module_type_from_uri`: корневые модули узнаются при родителе `Ext` или
`Configuration`; форма — по `…/Forms/<Форма>/Ext/Form/Module.bsl` или
`…/Forms/<Форма>/Module.bsl`, общая форма — `CommonForms/<Форма>/Ext/Form/Module.bsl`
или `CommonForms/<Форма>/Module.bsl`. Ветка «коллекция без `MdoType`»
получает таблицу известных, но не моделируемых коллекций
(`DocumentJournals`, `SettingsStorages`, `FilterCriteria`, `DocumentNumerators`,
`Sequences`, `Recalculations`, `ExternalDataSources`), чтобы `ObjectModule.bsl`
и `ManagerModule.bsl` без `Ext/` под ними получали тип, а произвольный файл с
таким именем вне коллекции — нет. Таблица тестов
`every_dictionary_module_name_classifies_case_insensitively` дополняется
EDT-парами.

`hir-def::module_index::parse_form_module_path` принимает обе формы;
`module_path_segment_modes` получает EDT-формы длины 2 (`Configuration/<файл>`),
3 (`<Коллекция>/<Имя>/<файл>`, `CommonForms/<Форма>/Module.bsl`), 5
(`…/Forms/<Форма>/Module.bsl`, `…/Commands/<Команда>/CommandModule.bsl`).
`ApplicationModuleKind::relative_path` дополняется `relative_path_in(format)`;
единственный не тестовый вызов в `ide-db::database` пробует обе формы.

### 5.5 CLI `bsl-analyzer` (Tier B, минимально)

`source_set.rs::root_configuration_xml` и парная проверка в `check_config.rs`
сводятся к `project_model::configuration_marker_in`; текст ошибки
`--configuration-root … no Configuration.xml at` упоминает оба маркера.
Ничего больше в крейте не меняется.

### 5.6 `mcp-server::state::sync`

Проверка «это XML метаданных рядом с `Configuration.xml`» расширяется на
`.mdo` рядом с `Configuration/Configuration.mdo`, иначе правка `.mdo` в
резиденте MCP не сбрасывала бы метаданные. Правка в две строки, покрыта
существующим тестом с добавленным случаем.

## 6. Поток данных

`bsl-analyzer analyze -s C:\…\empldocs-extension-edt\src\project\src`:

1. `Project::new(root)` → `discover_source_path` → в `root` есть
   `Configuration/Configuration.mdo`, `configuration_kind` видит
   `<configurationExtensionPurpose>` → корень записывается как
   `extension_in_config_location`, проект — расширение без базы (как сегодня
   для CFE-выгрузки без базы); предупреждение о базе печатается то же.
2. `WorkspaceConfigsSnapshot::from_project` регистрирует корень расширения.
3. `load_configuration(root)` → `load_from_directory` → `SourceFormat::Edt` →
   обнаружение по EDT-пробам → `parse_*_xml` → диспетчер EDT → модель.
   Строка `METADATA LOAD: … common_modules=203 metadata_objects=…` вместо
   нулей.
4. `build_module_metadata` для `CommonModules/лксОбщий/Module.bsl` → тип
   `CommonModule` (форма без `Ext/` уже разбирается `split_module_path`) →
   `find_common_module_by_path` по имени → контекст исполнения.
5. Для `Catalogs/X/Forms/Ф/Module.bsl` → `FormModule` → `parse_form_from_bsl_path`
   находит `Form.form` рядом → модель формы.

`analyze -s <расширение> --configuration-root C:\…\zup3-edt\src\project\src --extension <расширение>`:
оба корня проходят проверку по маркеру, оба загружаются как EDT, дальше
работает существующая цепочка `chain_configuration` со слиянием
заимствованных объектов.

## 7. Ошибки и логирование

- `.mdo`, который не разбирается, пропускается с `tracing::warn!` с путём и
  причиной, как сегодня пропускается битый `.xml`; загрузка не прерывается.
- Документ с неизвестным пространством имён (ни Designer, ни EDT) даёт
  `MetadataError::InvalidFormat` с текстом «unsupported metadata namespace
  …» — громко, чтобы новая версия формата не деградировала молча.
- В `load_from_directory` в существующий `tracing::info!("configuration loaded")`
  добавляется поле `format`; строка `METADATA LOAD` в `ide-db` не меняется
  (на неё завязан потребитель).
- Никаких `println!`; только `tracing`.

## 8. Тестирование

Фикстуры пишутся вручную по образцу `fixtures/designer`, с теми же
синтетическими именами (`Справочник1`, `Документ1`, `РегистрСведений1`,
`ПервыйОбщийМодуль`, …), чтобы тесты паритета сравнивали загрузку двух
раскладок одной конфигурации. Файлы конфигураций 1С в фикстуры не копируются.

`fixtures/edt/`: `Configuration/` (`Configuration.mdo`, `SessionModule.bsl`,
`ManagedApplicationModule.bsl`), `CommonModules/` (серверный глобальный,
клиентский, серверный с вызовом сервера), `Catalogs/Справочник1/` (реквизиты
всех базовых типов, ссылочный и составной тип, определяемый тип, владелец,
иерархия, табличная часть, предопределённые с вложенностью, формы элемента и
списка с `Form.form` и модулями, команда), `Documents/Документ1/` (движения
по регистрам, табличная часть, форма документа), `InformationRegisters/`
(периодический, с измерениями, ресурсами, реквизитами, модулем набора
записей), `AccumulationRegisters/`, `Enums/`, `Constants/` (с
`ValueManagerModule.bsl`), `DefinedTypes/`, `EventSubscriptions/`,
`ScheduledJobs/`, `Roles/` (с `Rights.rights`), `Subsystems/` (вложенная),
`HTTPServices/`, `WebServices/`, `CommonForms/`, `ChartsOfCharacteristicTypes/`.

`fixtures/edt_extension/`: `Configuration.mdo` с `configurationExtensionPurpose`
и `namePrefix`, заимствованный справочник с добавленным реквизитом,
заимствованный общий модуль с `Module.bsl`, собственный справочник с формой,
собственный общий модуль.

Тесты (все в `#[cfg(test)]` рядом с кодом, как принято в крейте):

- `layout`: детект по обоим маркерам и по их отсутствию, через `RealFs` и
  `PathSetTree`;
- `loader`: полная загрузка `fixtures/edt` даёт те же имена, типы реквизитов,
  табличные части, измерения, предопределённые, что и `fixtures/designer`
  (сравнение по подмножеству, которое есть в обеих фикстурах); обнаружение
  структуры совпадает между двумя источниками дерева; `discover_*` для EDT
  устойчивы к порядку `read_dir`;
- `xml_parser::edt`: по одному тесту на вид объекта с проверкой каждого
  читаемого свойства; типы — таблица токенов и квалификаторов; формы —
  элементы, родители, `dataPath`, обработчики, команды, реквизиты, главный
  реквизит; `ObjectBelonging` для обоих форматов;
- `project-model`: обнаружение EDT-корня и EDT-расширения, `--configuration-root`
  на EDT-каталог, роли файлов `.mdo`/`.form`/`.rights`;
- `ide-db`, `hir-def`: таблицы путей дополняются EDT-формами, включая
  регистронезависимость конвенционных сегментов и точность имён;
- `bsl-conventions`: снимок словаря, `path_is_root_module` в обеих формах.

Регрессия Designer: существующие тесты крейта не меняются по смыслу; тесты,
которые перечисляют словарь или снимок, обновляются ровно на добавленные
строки.

## 9. Приёмка и отчёт

Проверка бинарником из ветки на настоящих проектах, числа заносятся в отчёт:

1. `empldocs-extension-edt`: `common_modules` и `metadata_objects` ненулевые и
   сопоставимы с числом каталогов на диске (203 общих модуля, 1 352 `.mdo`).
2. Число срабатываний `UnresolvedField`, `QueryToMissingMetadata`,
   `UnresolvedMethodCall`, `UsingSynchronousCalls` до и после на том же
   проекте — отдельно и вместе с базой `zup3-edt` через `--configuration-root`.
3. `apard_xml`: 40 и 871 до и после, число срабатываний по правилам совпадает.
4. Загрузка расширения даёт объекты с `Adopted` и `Own` (проверяется тестом на
   фикстуре и точечно через MCP `metadata object` на живом расширении).
5. `cargo test --all` зелёный, `cargo clippy --all-targets --all-features -- -D warnings`
   чистый, `cargo fmt --all -- --check` чистый.
6. Ни одного флага формата в командной строке и в `bsl-analyzer.toml`.

Дополнительно: `zup3-edt` целиком (12 тысяч объектов) загружается без ошибок,
время загрузки записывается в отчёт как справочное.

## 10. Известные ограничения

- Отладчик (`bsl-debug`) ищет модули по раскладке Конфигуратора.
- MCP-инструмент `metadata forms` читает `Ext/Form.xml`.
- `bsl-search::context::file_path_to_module_path` строит подпись пути по
  Designer-сегментам; для EDT она получается той же для объектных модулей и
  отличается для форм (нет сегмента `Ext`, разбор идёт по `Forms/<Форма>`).
- Поиск корня в глубину два уровня не найдёт `src/project/src` от корня
  репозитория; корень указывается флагом `-s` или ключом `[source] root`.
- Защищённые модули в EDT не существуют, признак `protected` для EDT всегда
  `false`.

## 11. Происхождение кода

Источник истины для разбора — файлы реальных проектов EDT на диске и
документация 1С. Код сторонних разборщиков формата EDT (в том числе Java-
библиотек экосистемы `bsl-language-server`) при написании не открывался и не
открывается; это фиксируется в сообщении коммита, как того требует
`LICENSING.md` для нового кода крейта, находящегося под наблюдением.
