use crate::{
    Attribute, AttributeResource, AttributeValue, Content, Document, HtmlResource,
    HtmlResourceKind, SourceSetItem, SourceSetResource, assert_node,
};

use super::{TestParser, find_attribute, find_child_element};

/// Classify script, stylesheet, and asset attributes as structured resources.
#[test]
fn test_parse_document_classifies_single_value_resources() {
    let test = TestParser::new();
    let source = r#"
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
    "#;
    let (tree, document) = test.parse_document(source);

    assert_node!(tree, document, Document { children, .. } => {
        let html = find_child_element(&tree, children, "html");

        assert_node!(tree, html, Content::Element(html) => {
            let head = find_child_element(&tree, &html.children, "head");
            let body = find_child_element(&tree, &html.children, "body");

            assert_node!(tree, head, Content::Element(head) => {
                let link = find_child_element(&tree, &head.children, "link");
                let script = find_child_element(&tree, &head.children, "script");

                assert_node!(tree, link, Content::Element(link) => {
                    let href = find_attribute(&tree, &link.attributes, "href");

                    assert_node!(tree, href, Attribute {
                        value: Some(AttributeValue {
                            value,
                            resource: Some(AttributeResource::Resource(resource)),
                            ..
                        }),
                        ..
                    } => {
                        assert_eq!(value, "./styles.css");
                        assert_eq!(
                            resource,
                            &HtmlResource {
                                kind: HtmlResourceKind::Stylesheet,
                                path: "./styles.css".to_string(),
                                suffix: "".to_string(),
                                is_external: false,
                            }
                        );
                    });
                });

                assert_node!(tree, script, Content::Element(script) => {
                    let src = find_attribute(&tree, &script.attributes, "src");

                    assert_node!(tree, src, Attribute {
                        value: Some(AttributeValue {
                            value,
                            resource: Some(AttributeResource::Resource(resource)),
                            ..
                        }),
                        ..
                    } => {
                        assert_eq!(value, "./app.ts");
                        assert_eq!(
                            resource,
                            &HtmlResource {
                                kind: HtmlResourceKind::ModuleScript,
                                path: "./app.ts".to_string(),
                                suffix: "".to_string(),
                                is_external: false,
                            }
                        );
                    });
                });
            });

            assert_node!(tree, body, Content::Element(body) => {
                let image = find_child_element(&tree, &body.children, "img");

                assert_node!(tree, image, Content::Element(image) => {
                    let src = find_attribute(&tree, &image.attributes, "src");

                    assert_node!(tree, src, Attribute {
                        value: Some(AttributeValue {
                            value,
                            resource: Some(AttributeResource::Resource(resource)),
                            ..
                        }),
                        ..
                    } => {
                        assert_eq!(value, "./logo.svg");
                        assert_eq!(
                            resource,
                            &HtmlResource {
                                kind: HtmlResourceKind::Asset,
                                path: "./logo.svg".to_string(),
                                suffix: "".to_string(),
                                is_external: false,
                            }
                        );
                    });
                });
            });
        });
    });
}

/// Keep non-owned link references literal instead of classifying them as resources.
#[test]
fn test_parse_document_keeps_canonical_link_literal() {
    let test = TestParser::new();
    let source = r#"
        <!doctype html>
        <html>
            <head>
                <link rel="canonical" href="./canonical.html" />
            </head>
        </html>
    "#;
    let (tree, document) = test.parse_document(source);

    assert_node!(tree, document, Document { children, .. } => {
        let html = find_child_element(&tree, children, "html");

        assert_node!(tree, html, Content::Element(html) => {
            let head = find_child_element(&tree, &html.children, "head");

            assert_node!(tree, head, Content::Element(head) => {
                let link = find_child_element(&tree, &head.children, "link");

                assert_node!(tree, link, Content::Element(link) => {
                    let href = find_attribute(&tree, &link.attributes, "href");

                    assert_node!(tree, href, Attribute {
                        value: Some(AttributeValue {
                            value,
                            resource: None,
                            ..
                        }),
                        ..
                    } => {
                        assert_eq!(value, "./canonical.html");
                    });
                });
            });
        });
    });
}

/// Classify `srcset`-style values into structured candidate lists.
#[test]
fn test_parse_document_classifies_source_set_resources() {
    let test = TestParser::new();
    let source = r#"
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
    "#;
    let (tree, document) = test.parse_document(source);

    assert_node!(tree, document, Document { children, .. } => {
        let html = find_child_element(&tree, children, "html");

        assert_node!(tree, html, Content::Element(html) => {
            let head = find_child_element(&tree, &html.children, "head");

            assert_node!(tree, head, Content::Element(head) => {
                let link = find_child_element(&tree, &head.children, "link");

                assert_node!(tree, link, Content::Element(link) => {
                    let href = find_attribute(&tree, &link.attributes, "href");
                    let imagesrcset = find_attribute(&tree, &link.attributes, "imagesrcset");

                    assert_node!(tree, href, Attribute {
                        value: Some(AttributeValue {
                            resource: Some(AttributeResource::Resource(resource)),
                            ..
                        }),
                        ..
                    } => {
                        assert_eq!(
                            resource,
                            &HtmlResource {
                                kind: HtmlResourceKind::Asset,
                                path: "./hero.jpg".to_string(),
                                suffix: "".to_string(),
                                is_external: false,
                            }
                        );
                    });

                    assert_node!(tree, imagesrcset, Attribute {
                        value: Some(AttributeValue {
                            resource: Some(AttributeResource::SourceSet(resource)),
                            ..
                        }),
                        ..
                    } => {
                        assert_eq!(
                            resource,
                            &SourceSetResource {
                                items: vec![
                                    SourceSetItem {
                                        value: "./hero-small.jpg".to_string(),
                                        path: "./hero-small.jpg".to_string(),
                                        suffix: "".to_string(),
                                        descriptor: " 1x".to_string(),
                                        is_external: false,
                                    },
                                    SourceSetItem {
                                        value: "./hero-large.jpg".to_string(),
                                        path: "./hero-large.jpg".to_string(),
                                        suffix: "".to_string(),
                                        descriptor: " 2x".to_string(),
                                        is_external: false,
                                    },
                                ],
                            }
                        );
                    });
                });
            });
        });
    });
}
