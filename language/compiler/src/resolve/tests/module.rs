use super::*;
use destack_artifact::Data;
use destack_css::{Rule, Token};
use destack_dir::{StaticKey, SymbolSpace};
use destack_html::Content;
use destack_source::ModuleEdgeRelation;

/// Return whether one html name matches one expected local spelling.
fn html_name_is(html: &destack_artifact::Html, name: &destack_html::Name, expected: &str) -> bool {
    html.tree.strings.get(name.local) == expected
}

/// Return whether one css function name matches one expected spelling.
fn css_function_name_is(
    css: &destack_artifact::Css,
    function: &destack_css::Function,
    expected: &str,
) -> bool {
    css.tree
        .strings
        .get(function.name)
        .eq_ignore_ascii_case(expected)
}

/// Assert one module exports a type symbol for the requested name.
fn assert_has_type_export(test: &TestProgram, module_id: destack_source::ModuleId, name: &str) {
    let profile = test.default_profile_id(module_id);
    let dir = test.artifact_dir(module_id, profile);
    let exports = &dir.export_by_symbol_key;
    let name_id = test.program.strings.intern(name);
    let key = (SymbolSpace::Type, StaticKey::Name(name_id));

    let Some(export) = exports.get(&key) else {
        panic!("expected type export '{name}'");
    };
    assert!(
        export.symbol.is_some() || export.item.is_some(),
        "expected type export '{name}' to carry a target",
    );
}

/// Build module dependencies edges for import dependencies.
#[test]
fn test_module_dependencies_import_dependency() {
    let test = TestProgram::memory_sequential();
    let dep_source = r#"
export const value = 1;
"#;
    let main_source = r#"
import { value } from "./dep.ts";

value;
"#;

    let dep_module_id = test.add_module("dep.ts", dep_source);
    let main_module_id = test.add_module("main.ts", main_source);

    test.resolve_module(main_module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(main_module_id);
    let module_dependencies = test.module_dependencies(profile);
    let dependencies = module_dependencies.dependencies_for(main_module_id);

    // assert dependency edges
    assert!(
        dependencies.contains(&dep_module_id),
        "expected module dependencies to include dep.ts"
    );
}

/// Build module dependencies stylesheet edges for preload style links.
#[test]
fn test_module_dependencies_html_preload_style_dependency() {
    let test = TestProgram::memory_sequential();
    let stylesheet_module_id = test.add_module(
        "styles/site.css",
        r#"
body {
    color: red;
}
"#,
    );
    let html_module_id = test.add_module(
        "index.html",
        r#"
<!doctype html>
<html>
    <head>
        <link rel="preload" href="./styles/site.css" as="style" />
    </head>
</html>
"#,
    );

    test.resolve_module(html_module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(html_module_id);
    let module_dependencies = test.module_dependencies(profile);
    let dependencies = module_dependencies.dependencies_for(html_module_id);

    // assert dependency edges
    assert!(
        dependencies.contains(&stylesheet_module_id),
        "expected module dependencies to include stylesheet preload target",
    );
}

/// Record exact authored HTML edge relations and specifiers in the module dependencies.
#[test]
fn test_module_dependencies_html_records_exact_document_edge_specifiers() {
    let test = TestProgram::memory_sequential();
    let stylesheet_module_id = test.add_module(
        "src/styles/site.css",
        r#"
body {
    color: red;
}
"#,
    );
    let script_module_id = test.add_module(
        "src/scripts/app.ts",
        r#"
export const value = 1;
"#,
    );
    let asset_module_id = test.add_module("src/assets/logo.svg", "<svg></svg>");
    let html_module_id = test.add_module(
        "src/index.html",
        r#"
<!doctype html>
<html>
    <head>
        <link rel="stylesheet" href="/styles/site.css?v=1" />
        <script src="/scripts/app.ts?worker"></script>
    </head>
    <body>
        <img src="/assets/logo.svg#icon" />
    </body>
</html>
"#,
    );
    test.apply_destack_config(
        html_module_id,
        r#"
{
  "compiler": {
    "rootDir": "src"
  }
}
"#,
    );

    test.resolve_module(html_module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(html_module_id);
    let module_dependencies = test.module_dependencies(profile);
    let stylesheet_specifier = test.program.strings.intern("/styles/site.css?v=1");
    let script_specifier = test.program.strings.intern("/scripts/app.ts?worker");
    let asset_specifier = test.program.strings.intern("/assets/logo.svg#icon");

    assert_eq!(
        module_dependencies.dependency_target_for_specifier(
            html_module_id,
            ModuleEdgeRelation::DocumentStylesheet,
            stylesheet_specifier,
        ),
        Some(stylesheet_module_id),
    );
    assert_eq!(
        module_dependencies.dependency_target_for_specifier(
            html_module_id,
            ModuleEdgeRelation::DocumentScript,
            script_specifier,
        ),
        Some(script_module_id),
    );
    assert_eq!(
        module_dependencies.dependency_target_for_specifier(
            html_module_id,
            ModuleEdgeRelation::Resource,
            asset_specifier,
        ),
        Some(asset_module_id),
    );
}

/// Record exact HTML attribute sites in the module dependencies.
#[test]
fn test_module_dependencies_html_records_attribute_sites() {
    let test = TestProgram::memory_sequential();
    let stylesheet_module_id = test.add_module(
        "styles.css",
        r#"
body {
    color: red;
}
"#,
    );
    let script_module_id = test.add_module(
        "app.ts",
        r#"
export const value = 1;
"#,
    );
    let asset_module_id = test.add_module("logo.svg", "<svg></svg>");
    let html_module_id = test.add_module(
        "index.html",
        r#"
<!doctype html>
<html>
    <head>
        <link rel="stylesheet" href="./styles.css" />
        <script src="./app.ts"></script>
    </head>
    <body>
        <img src="./logo.svg" />
    </body>
</html>
"#,
    );

    test.resolve_module(html_module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(html_module_id);
    let module_dependencies = test.module_dependencies(profile);
    let data = test.data(html_module_id);
    let html = match data.as_ref() {
        Data::Html(html) => html,
        Data::Json(_) | Data::Css(_) => panic!("expected html data payload"),
    };
    let document = html.tree.get(html.document);
    let html_element = document
        .children
        .iter()
        .find_map(|node| match html.tree.get(*node) {
            Content::Element(element) if html_name_is(html, &element.name, "html") => Some(element),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing html element"));
    let head_element = html_element
        .children
        .iter()
        .find_map(|node| match html.tree.get(*node) {
            Content::Element(element) if html_name_is(html, &element.name, "head") => Some(element),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing head element"));
    let body_element = html_element
        .children
        .iter()
        .find_map(|node| match html.tree.get(*node) {
            Content::Element(element) if html_name_is(html, &element.name, "body") => Some(element),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing body element"));
    let stylesheet_element = head_element
        .children
        .iter()
        .find_map(|node| match html.tree.get(*node) {
            Content::Element(element) if html_name_is(html, &element.name, "link") => Some(element),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing link element"));
    let script_element = head_element
        .children
        .iter()
        .find_map(|node| match html.tree.get(*node) {
            Content::Element(element) if html_name_is(html, &element.name, "script") => {
                Some(element)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing script element"));
    let image_element = body_element
        .children
        .iter()
        .find_map(|node| match html.tree.get(*node) {
            Content::Element(element) if html_name_is(html, &element.name, "img") => Some(element),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing img element"));
    let stylesheet_attribute_id = *stylesheet_element
        .attributes
        .iter()
        .find(|attribute_id| html_name_is(html, &html.tree.get(**attribute_id).name, "href"))
        .unwrap_or_else(|| panic!("missing link href attribute"));
    let script_attribute_id = *script_element
        .attributes
        .iter()
        .find(|attribute_id| html_name_is(html, &html.tree.get(**attribute_id).name, "src"))
        .unwrap_or_else(|| panic!("missing script src attribute"));
    let image_attribute_id = *image_element
        .attributes
        .iter()
        .find(|attribute_id| html_name_is(html, &html.tree.get(**attribute_id).name, "src"))
        .unwrap_or_else(|| panic!("missing img src attribute"));
    let stylesheet_specifier = test.program.strings.intern("./styles.css");
    let script_specifier = test.program.strings.intern("./app.ts");
    let asset_specifier = test.program.strings.intern("./logo.svg");

    assert_eq!(
        module_dependencies
            .dependency_edge_for_site_specifier(
                html_module_id,
                ModuleEdgeRelation::DocumentStylesheet,
                stylesheet_attribute_id.id,
                stylesheet_specifier,
            )
            .map(|edge| edge.target),
        Some(stylesheet_module_id),
    );
    assert_eq!(
        module_dependencies
            .dependency_edge_for_site_specifier(
                html_module_id,
                ModuleEdgeRelation::DocumentScript,
                script_attribute_id.id,
                script_specifier,
            )
            .map(|edge| edge.target),
        Some(script_module_id),
    );
    assert_eq!(
        module_dependencies
            .dependency_edge_for_site_specifier(
                html_module_id,
                ModuleEdgeRelation::Resource,
                image_attribute_id.id,
                asset_specifier,
            )
            .map(|edge| edge.target),
        Some(asset_module_id),
    );
}

/// Keep owned link href relations narrow while preserving intended asset references.
#[test]
fn test_module_dependencies_html_link_asset_policy() {
    let test = TestProgram::memory_sequential();
    let canonical_module_id =
        test.add_module("canonical.html", "<!doctype html><title>Canonical</title>");
    let manifest_module_id = test.add_module("site.webmanifest", r#"{"name":"Site"}"#);
    let icon_module_id = test.add_module("favicon.svg", "<svg></svg>");
    let image_module_id = test.add_module("hero.jpg", "fake image content");
    let html_module_id = test.add_module(
        "index.html",
        r#"
<!doctype html>
<html>
    <head>
        <link rel="canonical" href="./canonical.html" />
        <link rel="manifest" href="./site.webmanifest" />
        <link rel="icon" href="./favicon.svg" />
        <link rel="preload" href="./hero.jpg" as="image" />
    </head>
</html>
"#,
    );

    test.resolve_module(html_module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(html_module_id);
    let module_dependencies = test.module_dependencies(profile);
    let dependencies = module_dependencies.dependencies_for(html_module_id);
    let data = test.data(html_module_id);
    let html = match data.as_ref() {
        Data::Html(html) => html,
        Data::Json(_) | Data::Css(_) => panic!("expected html data payload"),
    };
    let document = html.tree.get(html.document);
    let html_element = document
        .children
        .iter()
        .find_map(|node| match html.tree.get(*node) {
            Content::Element(element) if html_name_is(html, &element.name, "html") => Some(element),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing html element"));
    let head_element = html_element
        .children
        .iter()
        .find_map(|node| match html.tree.get(*node) {
            Content::Element(element) if html_name_is(html, &element.name, "head") => Some(element),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing head element"));
    let canonical_link = head_element
        .children
        .iter()
        .find_map(|node| match html.tree.get(*node) {
            Content::Element(element)
                if html_name_is(html, &element.name, "link")
                    && element.attributes.iter().any(|attribute_id| {
                        let attribute = html.tree.get(*attribute_id);
                        html_name_is(html, &attribute.name, "rel")
                            && attribute
                                .value
                                .as_ref()
                                .is_some_and(|value| value.value == "canonical")
                    }) =>
            {
                Some(element)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing canonical link element"));
    let _canonical_href_attribute = *canonical_link
        .attributes
        .iter()
        .find(|attribute_id| html_name_is(html, &html.tree.get(**attribute_id).name, "href"))
        .unwrap_or_else(|| panic!("missing canonical href attribute"));

    assert!(!dependencies.contains(&canonical_module_id));
    assert!(dependencies.contains(&manifest_module_id));
    assert!(dependencies.contains(&icon_module_id));
    assert!(dependencies.contains(&image_module_id));
}

/// Record `imagesrcset` sites on link preload elements as owned asset references.
#[test]
fn test_module_dependencies_html_records_link_imagesrcset_sites() {
    let test = TestProgram::memory_sequential();
    let fallback_module_id = test.add_module("hero.jpg", "fallback image");
    let small_module_id = test.add_module("hero-small.jpg", "small image");
    let large_module_id = test.add_module("hero-large.jpg", "large image");
    let html_module_id = test.add_module(
        "index.html",
        r#"
<!doctype html>
<html>
    <head>
        <link
            rel="preload"
            href="./hero.jpg"
            as="image"
            imagesrcset="./hero-small.jpg 1x, ./hero-large.jpg 2x"
        />
    </head>
</html>
"#,
    );

    test.resolve_module(html_module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(html_module_id);
    let module_dependencies = test.module_dependencies(profile);
    let data = test.data(html_module_id);
    let html = match data.as_ref() {
        Data::Html(html) => html,
        Data::Json(_) | Data::Css(_) => panic!("expected html data payload"),
    };
    let document = html.tree.get(html.document);
    let html_element = document
        .children
        .iter()
        .find_map(|node| match html.tree.get(*node) {
            Content::Element(element) if html_name_is(html, &element.name, "html") => Some(element),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing html element"));
    let head_element = html_element
        .children
        .iter()
        .find_map(|node| match html.tree.get(*node) {
            Content::Element(element) if html_name_is(html, &element.name, "head") => Some(element),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing head element"));
    let preload_element = head_element
        .children
        .iter()
        .find_map(|node| match html.tree.get(*node) {
            Content::Element(element) if html_name_is(html, &element.name, "link") => Some(element),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing preload link element"));
    let href_attribute_id = *preload_element
        .attributes
        .iter()
        .find(|attribute_id| html_name_is(html, &html.tree.get(**attribute_id).name, "href"))
        .unwrap_or_else(|| panic!("missing link href attribute"));
    let imagesrcset_attribute_id = *preload_element
        .attributes
        .iter()
        .find(|attribute_id| html_name_is(html, &html.tree.get(**attribute_id).name, "imagesrcset"))
        .unwrap_or_else(|| panic!("missing link imagesrcset attribute"));
    let href_specifier = test.program.strings.intern("./hero.jpg");
    let small_specifier = test.program.strings.intern("./hero-small.jpg");
    let large_specifier = test.program.strings.intern("./hero-large.jpg");

    assert_eq!(
        module_dependencies
            .dependency_edge_for_site_specifier(
                html_module_id,
                ModuleEdgeRelation::Resource,
                href_attribute_id.id,
                href_specifier,
            )
            .map(|edge| edge.target),
        Some(fallback_module_id),
    );
    assert_eq!(
        module_dependencies
            .dependency_edge_for_site_specifier(
                html_module_id,
                ModuleEdgeRelation::Resource,
                imagesrcset_attribute_id.id,
                small_specifier,
            )
            .map(|edge| edge.target),
        Some(small_module_id),
    );
    assert_eq!(
        module_dependencies
            .dependency_edge_for_site_specifier(
                html_module_id,
                ModuleEdgeRelation::Resource,
                imagesrcset_attribute_id.id,
                large_specifier,
            )
            .map(|edge| edge.target),
        Some(large_module_id),
    );
}

/// Build module dependencies edges for root-relative HTML script and stylesheet references.
#[test]
fn test_module_dependencies_html_root_relative_dependencies() {
    let test = TestProgram::memory_sequential();
    let stylesheet_module_id = test.add_module(
        "src/styles/site.css",
        r#"
body {
    color: red;
}
"#,
    );
    let script_module_id = test.add_module(
        "src/scripts/app.ts",
        r#"
export const value = 1;
"#,
    );
    let html_module_id = test.add_module(
        "src/index.html",
        r#"
<!doctype html>
<html>
    <head>
        <link rel="stylesheet" href="/styles/site.css" />
        <script src="/scripts/app.ts"></script>
    </head>
</html>
"#,
    );
    test.apply_destack_config(
        html_module_id,
        r#"
{
  "compiler": {
    "rootDir": "src"
  }
}
"#,
    );

    test.resolve_module(html_module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(html_module_id);
    let module_dependencies = test.module_dependencies(profile);
    let dependencies = module_dependencies.dependencies_for(html_module_id);

    // assert dependency edges
    assert!(
        dependencies.contains(&stylesheet_module_id),
        "expected module dependencies to include root-relative stylesheet target",
    );

    // assert dependency edges
    assert!(
        dependencies.contains(&script_module_id),
        "expected module dependencies to include root-relative script target",
    );
}

/// Record exact authored CSS import and url edge specifiers in the module dependencies.
#[test]
fn test_module_dependencies_css_records_exact_edge_specifiers() {
    let test = TestProgram::memory_sequential();
    let imported_stylesheet_module_id = test.add_module(
        "styles/reset.css",
        r#"
html {
    box-sizing: border-box;
}
"#,
    );
    let asset_module_id = test.add_module("images/pattern.svg", "<svg></svg>");
    let stylesheet_module_id = test.add_module(
        "styles/site.css",
        r#"
@import "./reset.css?inline";

body {
    background-image: url("../images/pattern.svg#hero");
}
"#,
    );

    test.resolve_module(stylesheet_module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(stylesheet_module_id);
    let module_dependencies = test.module_dependencies(profile);
    let data = test.data(stylesheet_module_id);
    let css = match data.as_ref() {
        Data::Css(css) => css,
        Data::Json(_) | Data::Html(_) => panic!("expected css data payload"),
    };
    let import_specifier = test.program.strings.intern("./reset.css?inline");
    let asset_specifier = test.program.strings.intern("../images/pattern.svg#hero");
    let import_site_id = css
        .tree
        .get(css.stylesheet)
        .rules
        .iter()
        .find_map(|rule_id| match css.tree.get(*rule_id) {
            Rule::Import(rule) if rule.url == "./reset.css?inline" => {
                rule.resource.as_ref().map(|resource| resource.id)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing css import resource"));
    let asset_site_id = css
        .tree
        .get(css.stylesheet)
        .rules
        .iter()
        .find_map(|rule_id| {
            let Rule::Style(rule) = css.tree.get(*rule_id) else {
                return None;
            };
            let declarations = rule.declarations?;
            let declarations = css.tree.get(declarations);

            for declaration_id in &declarations.declarations {
                let declaration = css.tree.get(*declaration_id);

                for value in &declaration.value.components().values {
                    match value {
                        destack_css::ComponentValue::Token(Token::UnquotedUrl {
                            value,
                            url_resource,
                        }) if value == "../images/pattern.svg#hero" => {
                            return url_resource.as_ref().map(|resource| resource.id);
                        }
                        destack_css::ComponentValue::Function(function)
                            if css_function_name_is(css, function, "url") =>
                        {
                            if function.url_resource.is_some() {
                                return function.url_resource.as_ref().map(|resource| resource.id);
                            }
                        }
                        _ => {}
                    }
                }
            }

            None
        })
        .unwrap_or_else(|| panic!("missing css url resource"));

    assert_eq!(
        module_dependencies.dependency_target_for_specifier(
            stylesheet_module_id,
            ModuleEdgeRelation::StyleImport,
            import_specifier,
        ),
        Some(imported_stylesheet_module_id),
    );
    assert_eq!(
        module_dependencies.dependency_target_for_specifier(
            stylesheet_module_id,
            ModuleEdgeRelation::StyleUrl,
            asset_specifier,
        ),
        Some(asset_module_id),
    );
    assert_eq!(
        module_dependencies
            .dependency_edge_for_site_specifier(
                stylesheet_module_id,
                ModuleEdgeRelation::StyleImport,
                import_site_id,
                import_specifier,
            )
            .map(|edge| edge.target),
        Some(imported_stylesheet_module_id),
    );
    assert_eq!(
        module_dependencies
            .dependency_edge_for_site_specifier(
                stylesheet_module_id,
                ModuleEdgeRelation::StyleUrl,
                asset_site_id,
                asset_specifier,
            )
            .map(|edge| edge.target),
        Some(asset_module_id),
    );
}

/// Resolve ts relative .js specifiers through TypeScript extension substitution.
#[test]
fn test_module_dependencies_typescript_import_js_specifier_dependency() {
    let test = TestProgram::memory_sequential();
    let dep_source = r#"
export const value = 1;
"#;
    let main_source = r#"
import { value } from "./dep.js";

value;
"#;

    let dep_module_id = test.add_module("dep.ts", dep_source);
    let main_module_id = test.add_module("main.ts", main_source);

    test.resolve_module(main_module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(main_module_id);
    let module_dependencies = test.module_dependencies(profile);
    let dependencies = module_dependencies.dependencies_for(main_module_id);

    // assert dependency edges
    assert!(
        dependencies.contains(&dep_module_id),
        "expected module dependencies to include dep.ts for ./dep.js import"
    );
}

/// Publish prepared type exports for exported type aliases.
#[test]
fn test_module_exports_type_alias_symbol() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "mod.ds",
        r#"
export type User = { name: string };
"#,
    );

    test.resolve_module(module_id);
    test.compile_check_clean();

    assert_has_type_export(&test, module_id, "User");
}

/// Publish prepared default type exports for exported interfaces.
#[test]
fn test_module_exports_default_interface_symbol() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "mod.ds",
        r#"
export default interface User {
    name: string;
}
"#,
    );

    test.resolve_module(module_id);
    test.compile_check_clean();

    assert_has_type_export(&test, module_id, "default");
}

/// Resolve declaration imports with .js specifiers through declaration targets.
#[test]
fn test_module_dependencies_declaration_import_js_specifier_dependency() {
    let test = TestProgram::memory_sequential();
    let dep_source = r#"
export type TaskResultPack = { ok: true };
"#;
    let main_source = r#"
import { TaskResultPack } from "./dep.js";

type Wrapped = TaskResultPack;
"#;

    let dep_module_id = test.add_module("dep.d.ts", dep_source);
    let main_module_id = test.add_module("main.d.ts", main_source);

    test.resolve_module(main_module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(main_module_id);
    let module_dependencies = test.module_dependencies(profile);
    let dependencies = module_dependencies.dependencies_for(main_module_id);

    // assert dependency edges
    assert!(
        dependencies.contains(&dep_module_id),
        "expected module dependencies to include dep.d.ts for ./dep.js import"
    );
}

/// Resolve declaration reexports that rename type-only exports from .js specifiers.
#[test]
fn test_module_dependencies_declaration_reexport_type_alias_from_js_specifier() {
    let test = TestProgram::memory_sequential();
    let task_source = r#"
export interface TaskResultPack {
    ok: true;
}

export { type TaskResultPack as K };
"#;
    let index_source = r#"
export { K as TaskResultPack } from "./tasks.js";
"#;
    let main_source = r#"
import { TaskResultPack } from "./index.js";

type Wrapped = TaskResultPack;
"#;

    test.add_module("tasks.d.ts", task_source);
    test.add_module("index.d.ts", index_source);
    let main_module_id = test.add_module("main.d.ts", main_source);

    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Resolve declaration export-from when the module also imports from the same target.
#[test]
fn test_module_dependencies_declaration_export_from_with_sibling_import() {
    let test = TestProgram::memory_sequential();
    let tasks_source = r#"
export interface TaskResult {
    state: string;
}

export type TaskResultPack = TaskResult[];

export { type TaskResult as J, type TaskResultPack as K };
"#;
    let runner_source = r#"
import { J as TaskResult } from "./tasks.js";

export { J as TaskResult, K as TaskResultPack } from "./tasks.js";
"#;
    let main_source = r#"
import { TaskResultPack } from "./runner.js";

type Wrapped = TaskResultPack;
"#;

    test.add_module("tasks.d.ts", tasks_source);
    test.add_module("runner.d.ts", runner_source);
    let main_module_id = test.add_module("main.d.ts", main_source);

    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Keep node strict self package behavior for packages without exports.
#[test]
fn test_module_dependencies_self_package_import_without_exports_reports_error() {
    let test = TestProgram::memory_sequential();

    // build absolute test paths so package discovery can locate package.json
    let root = std::env::current_dir().expect("failed to read current directory");
    let package_json_path = root.join("package.json");
    let consumer_path = root.join("src/consumer.ts");

    // configure package metadata without exports
    test.add_file(
        &package_json_path.to_string_lossy(),
        r#"{ "name": "pkg", "main": "./src/index.ts" }"#,
    );

    // import package self name from package source
    let consumer_module_id = test.add_module(
        &consumer_path.to_string_lossy(),
        r#"
import { value } from "pkg";

value;
"#,
    );

    // resolve module and assert strict self import failure
    test.resolve_module(consumer_module_id);
    test.compile();
    test.check_has_diagnostic("ER200");
}

/// Build module dependencies edges for self package imports with exports.
#[test]
fn test_module_dependencies_self_package_import_with_exports_dependency() {
    let test = TestProgram::memory_sequential();

    // build absolute test paths so package discovery can locate package.json
    let root = std::env::current_dir().expect("failed to read current directory");
    let package_json_path = root.join("package.json");
    let package_entry_path = root.join("src/index.ts");
    let consumer_path = root.join("src/consumer.ts");

    // configure package metadata with explicit exports root
    test.add_file(
        &package_json_path.to_string_lossy(),
        r#"{ "name": "pkg", "main": "./src/main.ts", "exports": { ".": "./src/index.ts" } }"#,
    );

    // define exported entry module and consumer
    let package_entry_module_id = test.add_module(
        &package_entry_path.to_string_lossy(),
        r#"
export const value = 1;
"#,
    );
    let consumer_module_id = test.add_module(
        &consumer_path.to_string_lossy(),
        r#"
import { value } from "pkg";

value;
"#,
    );

    // resolve dependencies and ensure dependency targets exports root
    test.resolve_module(consumer_module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(consumer_module_id);
    let module_dependencies = test.module_dependencies(profile);
    let dependencies = module_dependencies.dependencies_for(consumer_module_id);

    // assert dependency edges
    assert!(
        dependencies.contains(&package_entry_module_id),
        "expected module dependencies to include src/index.ts for exported self import"
    );
}

/// Build module dependencies edges for namespace exports.
#[test]
fn test_module_dependencies_namespace_export_dependency() {
    let test = TestProgram::memory_sequential();
    let dep_source = r#"
export const value = 1;
"#;
    let export_source = r#"
export * from "./dep.ts";
"#;

    let dep_module_id = test.add_module("dep.ts", dep_source);
    let export_module_id = test.add_module("reexport.ts", export_source);

    test.resolve_module(export_module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(export_module_id);
    let module_dependencies = test.module_dependencies(profile);
    let dependencies = module_dependencies.dependencies_for(export_module_id);

    // assert dependency edges
    assert!(
        dependencies.contains(&dep_module_id),
        "expected module dependencies to include dep.ts"
    );
}

/// Build module dependencies edges for module binding imports.
#[test]
fn test_module_dependencies_binding_dependency() {
    let test = TestProgram::memory_sequential();
    let decl_source = r#"
declare module "foo" {
    export const value: number;
}
"#;
    let main_source = r#"
import { value } from "foo";

value;
"#;

    let decl_module_id = test.add_module("decl.d.ts", decl_source);
    let main_module_id = test.add_module("main.ts", main_source);

    test.import_module(decl_module_id);
    test.compile_check_clean();

    test.resolve_module(main_module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(main_module_id);
    let module_dependencies = test.module_dependencies(profile);
    let dependencies = module_dependencies.dependencies_for(main_module_id);

    // assert dependency edges
    assert!(
        dependencies.contains(&decl_module_id),
        "expected module dependencies to include module binding module"
    );
}
