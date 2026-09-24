use crate::assert_format_program;
use destack_source::FileType;

#[test]
fn test_format_symbolic_unary_prefix_chains() {
    assert_format_program!(
        r#"const notReady = ! !ready
const bits = ~ ~mask
const pre = ++ count
const post = count ++
"#,
        r#"const notReady = !!ready;
const bits = ~~mask;
const pre = ++count;
const post = count++;
"#,
        FileType::Destack
    );
}

#[test]
fn test_format_symbolic_unary_prefix_token_collisions() {
    assert_format_program!(
        r#"const positive = + +value
const negative = - -value
const plusIncrement = + ++value
const minusDecrement = - --value
"#,
        r#"const positive = + +value;
const negative = - -value;
const plusIncrement = + ++value;
const minusDecrement = - --value;
"#,
        FileType::Destack
    );
}

#[test]
fn test_format_await_prefix_forms() {
    assert_format_program!(
        r#"async function run() {
    const value = await   load()
    const maybe = await?   loadMaybe()
    const must = await!   loadMust()
}
"#,
        r#"async function run() {
    const value = await load();
    const maybe = await? loadMaybe();
    const must = await! loadMust();
}
"#,
        FileType::Destack
    );
}

#[test]
fn test_format_borrow_reference_chain_compact() {
    assert_format_program!(
        r#"const borrowed = & &value
"#,
        r#"const borrowed = &&value;
"#,
        FileType::Destack
    );
}

#[test]
fn test_format_borrow_reference_prefix_modifiers() {
    assert_format_program!(
        r#"const borrowed = & readonly super value
"#,
        r#"const borrowed = &readonly super value;
"#,
        FileType::Destack
    );
}

/// Prefix memory operators should preserve binary operands as one value.
#[test]
fn test_format_borrow_reference_binary_operands() {
    assert_format_program!(
        r#"const borrowed = &(left+right)
"#,
        r#"const borrowed = &(left + right);
"#,
        FileType::Destack
    );
}

#[test]
fn test_format_type_reference_chain_compact() {
    assert_format_program!(
        r#"type Borrowed = & &Buffer
type Moved = ^ ^Buffer
type BorrowMove = & ^Buffer
type MoveBorrow = ^ &Buffer
"#,
        r#"type Borrowed = &&Buffer;
type Moved = ^^Buffer;
type BorrowMove = &^Buffer;
type MoveBorrow = ^&Buffer;
"#,
        FileType::Destack
    );
}

#[test]
fn test_format_type_reference_prefix_modifiers() {
    assert_format_program!(
        r#"type Borrowed = & readonly super Buffer
type Moved = ^ exclusive extends Buffer
"#,
        r#"type Borrowed = &readonly super Buffer;
type Moved = ^exclusive extends Buffer;
"#,
        FileType::Destack
    );
}

#[test]
fn test_format_type_prefix_operators() {
    assert_format_program!(
        r#"type Keys = keyof   Model
type Query = typeof   value
type Negative = ! !Flag
"#,
        r#"type Keys = keyof Model;
type Query = typeof value;
type Negative = !!Flag;
"#,
        FileType::Destack
    );
}
