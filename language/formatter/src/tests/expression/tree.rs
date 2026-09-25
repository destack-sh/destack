use crate::{
    TsppFormatOptions, assert_format_program, assert_format_program_reference_widths,
    assert_format_roundtrip, parse_first_expression,
};
use tspp_source::FileType;

/// Tree attribute kinds should stay in a stable opening tag order and spelling.
#[test]
fn test_format_tree_attribute_kinds() {
    assert_format_program!(
        r#"const view=<Button disabled title="Save" count={items.length} {...props}/>
"#,
        r#"const view = <Button disabled title="Save" count={items.length} {...props} />;
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(100).with_indent_width(2)
    );
}

/// Malformed tree attributes should retain their slot and following attributes.
#[test]
fn test_format_recovered_tree_attribute() {
    assert_format_roundtrip!(
        r#"<Panel broken= next="ok" />"#,
        r#"<Panel broken= next="ok" />"#,
        FileType::Tspp,
        parse_first_expression,
    );
}

/// Malformed tree children should retain their slot and following children.
#[test]
fn test_format_recovered_tree_child() {
    assert_format_roundtrip!(
        "<Panel>{,}<Child /></Panel>",
        r#"<Panel>
    {,}
    <Child />
</Panel>"#,
        FileType::Tspp,
        parse_first_expression,
    );
}

/// Missing closing tags should be restored from the recovered tree structure.
#[test]
fn test_format_recovered_tree_closing_tag() {
    assert_format_roundtrip!(
        "<Panel><Child />",
        r#"<Panel>
    <Child />
</Panel>"#,
        FileType::Tspp,
        parse_first_expression,
    );
}

/// Tree attributes should break predictably when the opening tag no longer fits.
#[test]
fn test_format_tree_attributes_break_by_width() {
    assert_format_program_reference_widths(
        r#"const view = <Button disabled title="Save" count={items.length} onClick={() => submit(items)} {...props} />
"#,
        FileType::Tspp,
        &[
            (
                80,
                r#"const view = (
  <Button
    disabled
    title="Save"
    count={items.length}
    onClick={() => submit(items)}
    {...props}
  />
);
"#,
            ),
            (
                140,
                r#"const view = <Button disabled title="Save" count={items.length} onClick={() => submit(items)} {...props} />;
"#,
            ),
        ],
    );
}

/// Nested tree children should use the standard one-child-per-line element layout.
#[test]
fn test_format_tree_nested_children() {
    assert_format_program!(
        r#"const view=<Panel><Section key={item.id}><Item value={item.value}/></Section><Footer>{summary}</Footer></Panel>
"#,
        r#"const view = (
  <Panel>
    <Section key={item.id}>
      <Item value={item.value} />
    </Section>
    <Footer>{summary}</Footer>
  </Panel>
);
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(100).with_indent_width(2)
    );
}

/// Inline tree text and expression children should preserve word spacing.
#[test]
fn test_format_tree_inline_text_and_expression_children() {
    assert_format_program!(
        r#"const view=<p>Hello {name}!{" "}<strong>{count}</strong></p>
"#,
        r#"const view = <p>Hello {name}! <strong>{count}</strong></p>;
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(100).with_indent_width(2)
    );
}

/// Inline tree prose should preserve a multiline embedded element body.
#[test]
fn test_format_tree_inline_prose_preserves_multiline_element_body() {
    assert_format_program!(
        r#"const view=<p>See <Link>
  Docs
</Link> now.</p>
"#,
        r#"const view = (
  <p>
    See <Link>
      Docs
    </Link> now.
  </p>
);
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(100).with_indent_width(2)
    );
}

/// Tree fragments should format like ordinary tree containers.
#[test]
fn test_format_tree_fragment_children() {
    assert_format_program!(
        r#"const view=<><Item key={items[0].id}>{items[0].name}</Item><Item key={items[1].id}>{items[1].name}</Item></>
"#,
        r#"const view = (
  <>
    <Item key={items[0].id}>{items[0].name}</Item>
    <Item key={items[1].id}>{items[1].name}</Item>
  </>
);
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(100).with_indent_width(2)
    );
}

/// Tree attribute expression containers may contain nested tree literals.
#[test]
fn test_format_tree_attribute_expression() {
    assert_format_program!(
        r#"const view=<Show when={ready} fallback={<EmptyState title="Nothing here"/>}><Content /></Show>
"#,
        r#"const view = (
  <Show when={ready} fallback={<EmptyState title="Nothing here" />}>
    <Content />
  </Show>
);
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(100).with_indent_width(2)
    );
}

/// Tree expression containers with comments should keep comments inside braces.
#[test]
fn test_format_tree_expression_child_comment() {
    assert_format_program!(
        r#"const view=<div>{/* explain */}</div>
"#,
        r#"const view = <div>{/* explain */}</div>;
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(100).with_indent_width(2)
    );
}
