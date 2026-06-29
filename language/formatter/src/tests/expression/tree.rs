use crate::{DestackFormatOptions, assert_format_program, assert_format_program_reference_widths};
use destack_source::FileType;

/// TSX attribute kinds should stay in a stable opening tag order and spelling.
#[test]
fn test_format_tsx_attribute_kinds() {
    assert_format_program!(
        r#"const view=<Button disabled title="Save" count={items.length} {...props}/>
"#,
        r#"const view = <Button disabled title="Save" count={items.length} {...props} />;
"#,
        FileType::TypeScriptXml,
        DestackFormatOptions::default_with_line_width(100).with_indent_width(2)
    );
}

/// TSX attributes should break predictably when the opening tag no longer fits.
#[test]
fn test_format_tsx_attributes_break_by_width() {
    assert_format_program_reference_widths(
        r#"const view = <Button disabled title="Save" count={items.length} onClick={() => submit(items)} {...props} />
"#,
        FileType::TypeScriptXml,
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

/// TSX nested children should use the standard one-child-per-line element layout.
#[test]
fn test_format_tsx_nested_children() {
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
        FileType::TypeScriptXml,
        DestackFormatOptions::default_with_line_width(100).with_indent_width(2)
    );
}

/// TSX inline text and expression children should preserve word spacing.
#[test]
fn test_format_tsx_inline_text_and_expression_children() {
    assert_format_program!(
        r#"const view=<p>Hello {name}!{" "}<strong>{count}</strong></p>
"#,
        r#"const view = <p>Hello {name}! <strong>{count}</strong></p>;
"#,
        FileType::TypeScriptXml,
        DestackFormatOptions::default_with_line_width(100).with_indent_width(2)
    );
}

/// TSX fragments should format like ordinary JSX containers.
#[test]
fn test_format_tsx_fragment_children() {
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
        FileType::TypeScriptXml,
        DestackFormatOptions::default_with_line_width(100).with_indent_width(2)
    );
}

/// TSX attribute expression containers may contain nested tree literals.
#[test]
fn test_format_tsx_tree_attribute_expression() {
    assert_format_program!(
        r#"const view=<Show when={ready} fallback={<EmptyState title="Nothing here"/>}><Content /></Show>
"#,
        r#"const view = (
  <Show when={ready} fallback={<EmptyState title="Nothing here" />}>
    <Content />
  </Show>
);
"#,
        FileType::TypeScriptXml,
        DestackFormatOptions::default_with_line_width(100).with_indent_width(2)
    );
}

/// TSX expression containers with comments should keep comments inside braces.
#[test]
fn test_format_tsx_expression_child_comment() {
    assert_format_program!(
        r#"const view=<div>{/* explain */}</div>
"#,
        r#"const view = <div>{/* explain */}</div>;
"#,
        FileType::TypeScriptXml,
        DestackFormatOptions::default_with_line_width(100).with_indent_width(2)
    );
}
