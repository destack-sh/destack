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
        FileType::TypeScript
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
        FileType::TypeScript
    );
}

#[test]
fn test_format_keyword_unary_prefix_spacing() {
    assert_format_program!(
        r#"const kind = typeof   value
const ignored = void   run()
"#,
        r#"const kind = typeof value;
const ignored = void run();
"#,
        FileType::TypeScript
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
fn test_format_value_reference_chain_compact() {
    assert_format_program!(
        r#"const borrowed = & &value
const moved = ^ ^value
const borrowMove = & ^value
const moveBorrow = ^ &value
"#,
        r#"const borrowed = &&value;
const moved = ^^value;
const borrowMove = &^value;
const moveBorrow = ^&value;
"#,
        FileType::Destack
    );
}

#[test]
fn test_format_value_reference_prefix_modifiers() {
    assert_format_program!(
        r#"const borrowed = & readonly super value
const moved = ^ exclusive super value
"#,
        r#"const borrowed = &readonly super value;
const moved = ^exclusive super value;
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
type LocalBox = local   ^ User
"#,
        r#"type Keys = keyof Model;
type Query = typeof value;
type Negative = !!Flag;
type LocalBox = local ^User;
"#,
        FileType::Destack
    );
}
