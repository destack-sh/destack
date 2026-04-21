use crate::assert_format_program_reference_widths;
use destack_source::FileType;

/// Type alias comments after `=` should normalize to the stable union shell.
#[test]
fn test_format_union_head_comment_after_equals_is_idempotent() {
    assert_format_program_reference_widths(
        r#"type Aa1 = /*1*/ | /*2*/ C | D;
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"type Aa1 = /*1*/ /*2*/ C | D;
"#,
            ),
            (
                100,
                r#"type Aa1 = /*1*/ /*2*/ C | D;
"#,
            ),
        ],
    );
}

/// Type alias line comments after `=` should keep the rhs in the assignment shell.
#[test]
fn test_format_type_alias_line_comment_after_equals() {
    assert_format_program_reference_widths(
        r#"type Item = // keep
Alpha | Beta;
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"type Item = // keep
  Alpha | Beta;
"#,
            ),
            (
                100,
                r#"type Item = // keep
  Alpha | Beta;
"#,
            ),
        ],
    );
}

/// Assignment comments should stay attached to the formatted assignment shell.
#[test]
fn test_format_assignment_comments() {
    assert_format_program_reference_widths(
        r#"var longlonglonglonglonglong = /*#__PURE__*/_interopDefaultLegacy(aaaaaaaaaaaaaaa);
var short = /*#__PURE__*/_interopDefaultLegacy(b);

const jestPackageJson =
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  require(jestPath);

class A {
  #testerConfig;
  constructor() {
    let basePath: string | undefined =
      this.#testerConfig.languageOptions.parserOptions?.tsconfigRootDir;
  }
}
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"var longlonglonglonglonglong =
  /*#__PURE__*/ _interopDefaultLegacy(aaaaaaaaaaaaaaa);
var short = /*#__PURE__*/ _interopDefaultLegacy(b);

const jestPackageJson =
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  require(jestPath);

class A {
  #testerConfig;
  constructor() {
    let basePath: string | undefined =
      this.#testerConfig.languageOptions.parserOptions?.tsconfigRootDir;
  }
}
"#,
            ),
            (
                100,
                r#"var longlonglonglonglonglong = /*#__PURE__*/ _interopDefaultLegacy(aaaaaaaaaaaaaaa);
var short = /*#__PURE__*/ _interopDefaultLegacy(b);

const jestPackageJson =
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  require(jestPath);

class A {
  #testerConfig;
  constructor() {
    let basePath: string | undefined =
      this.#testerConfig.languageOptions.parserOptions?.tsconfigRootDir;
  }
}
"#,
            ),
        ],
    );
}

/// Assignment comment wrappers should preserve explicit grouping around the rhs.
#[test]
fn test_format_assignment_comment_wrapper_preserves_parenthesized_rhs() {
    assert_format_program_reference_widths(
        r#"{
  sourcemap =
  /** @type {'inline' | 'hidden' | 'sourcemap'} */ (
      process.env.WORKER_MODE
    ) || sourcemap;
}
"#,
        FileType::TypeScript,
        &[(
            80,
            r#"{
  sourcemap =
    /** @type {'inline' | 'hidden' | 'sourcemap'} */ (
      process.env.WORKER_MODE
    ) || sourcemap;
}
"#,
        )],
    );
}

/// CommonJS require initializers should keep the call attached to `=`.
#[test]
fn test_format_assignment_require_initializer_stays_attached() {
    assert_format_program_reference_widths(
        r#"const veryLongPackageBindingName = require(jestPath)
"#,
        FileType::TypeScript,
        &[(
            30,
            r#"const veryLongPackageBindingName = require(
  jestPath,
);
"#,
        )],
    );
}

/// Interpolated template arguments should not trigger the poorly-breakable call shortcut.
#[test]
fn test_format_assignment_interpolated_template_argument_uses_fluid_layout() {
    assert_format_program_reference_widths(
        r#"const veryLongBindingName = namespace.foo(`hello ${name}`)
"#,
        FileType::TypeScript,
        &[(
            30,
            r#"const veryLongBindingName =
  namespace.foo(
    `hello ${name}`,
  );
"#,
        )],
    );
}

/// Primitive `null` arguments should still count as short in the assignment-like chain shell.
#[test]
fn test_format_assignment_null_argument_breaks_after_operator() {
    assert_format_program_reference_widths(
        r#"const veryLongBindingName = namespace.foo(null)
"#,
        FileType::TypeScript,
        &[(
            30,
            r#"const veryLongBindingName =
  namespace.foo(null);
"#,
        )],
    );
}

/// Defaulted nested object patterns should stay flat within their assignment wrapper.
#[test]
fn test_format_assignment_nested_object_pattern_stays_inline() {
    assert_format_program_reference_widths(
        r#"const {
  data: { args: { something } } = {
    args: { something: []},
  }
} = obj;
"#,
        FileType::TypeScript,
        &[(
            80,
            r#"const {
  data: { args: { something } } = {
    args: { something: [] },
  },
} = obj;
"#,
        )],
    );
}

/// Call-expression type arguments should preserve leading comments.
#[test]
fn test_format_assignment_call_with_type_args_comments() {
    assert_format_program_reference_widths(
        r#"// Type arguments should not lose comments
export const globalRegistry: $ZodRegistry = /*@__PURE__*/ registry();

// Comments before call expressions with type arguments should be preserved
const r = /* THIS */ f<Type>()
const s = /* comment */ foo<A | B | C>()
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"// Type arguments should not lose comments
export const globalRegistry: $ZodRegistry = /*@__PURE__*/ registry();

// Comments before call expressions with type arguments should be preserved
const r = /* THIS */ f<Type>();
const s = /* comment */ foo<A | B | C>();
"#,
            ),
            (
                100,
                r#"// Type arguments should not lose comments
export const globalRegistry: $ZodRegistry = /*@__PURE__*/ registry();

// Comments before call expressions with type arguments should be preserved
const r = /* THIS */ f<Type>();
const s = /* comment */ foo<A | B | C>();
"#,
            ),
        ],
    );
}

/// Conditional type aliases should keep the assignment-like shell from the reference formatter.
#[test]
fn test_format_type_alias_conditional_layout() {
    assert_format_program_reference_widths(
        r#"export type _Repeat<A extends any, N extends number, L extends List = []> =
  __Repeat<N, A, L> extends infer X
  ? Cast<X, List>
  : never
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"export type _Repeat<A extends any, N extends number, L extends List = []> =
  __Repeat<N, A, L> extends infer X ? Cast<X, List> : never;
"#,
            ),
            (
                100,
                r#"export type _Repeat<A extends any, N extends number, L extends List = []> =
  __Repeat<N, A, L> extends infer X ? Cast<X, List> : never;
"#,
            ),
        ],
    );
}

/// Assignment-like arrow functions with type-heavy left sides should follow the reference widths.
#[test]
fn test_format_assignment_arrow_function_layout() {
    assert_format_program_reference_widths(
        r#"{
  const onPanning: ComponenASDtProps<
    typeof TransformWrapper
  >["onPanning"] = () => {
  }
}
const onPanning: ComponenASDtProps<
  typeof TransformWrapper
>["onPanning"] = () => {
}
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"{
  const onPanning: ComponenASDtProps<
    typeof TransformWrapper
  >["onPanning"] = () => {};
}
const onPanning: ComponenASDtProps<
  typeof TransformWrapper
>["onPanning"] = () => {};
"#,
            ),
            (
                100,
                r#"{
  const onPanning: ComponenASDtProps<typeof TransformWrapper>["onPanning"] = () => {};
}
const onPanning: ComponenASDtProps<typeof TransformWrapper>["onPanning"] = () => {};
"#,
            ),
        ],
    );
}

/// Formats chained assignment initializers with the stepped shell.
#[test]
fn test_format_assignment_chain_layout() {
    assert_format_program_reference_widths(
        r#"const longVariableName = alpha = beta = computeValue()
"#,
        FileType::TypeScript,
        &[(
            30,
            r#"const longVariableName =
  (alpha =
  beta =
    computeValue());
"#,
        )],
    );
}

/// Formats nested assignment initializers with explicit parentheses.
#[test]
fn test_format_assignment_chain_lambda_tail_layout() {
    assert_format_program_reference_widths(
        r#"const longVariableName = alpha = beta = () => {}
const short = a = b
"#,
        FileType::TypeScript,
        &[(
            30,
            r#"const longVariableName =
  (alpha =
  beta =
    () => {});
const short = (a = b);
"#,
        )],
    );
}

/// Complex destructuring assignments should use the reference break-left-hand-side layout.
#[test]
fn test_format_assignment_break_left_hand_side_layout() {
    assert_format_program_reference_widths(
        r#"{
  let { className, unfurl: unfurlAttrr, ...attrs } = getAttributesFromNode(node);

  ({ className, unfurl: unfurlAttrr, ...attrs } = { className: "name", unfurl: "unfurl", others: [1, 2, 3]});
};
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"{
  let {
    className,
    unfurl: unfurlAttrr,
    ...attrs
  } = getAttributesFromNode(node);

  ({
    className,
    unfurl: unfurlAttrr,
    ...attrs
  } = { className: "name", unfurl: "unfurl", others: [1, 2, 3] });
}
"#,
            ),
            (
                100,
                r#"{
  let { className, unfurl: unfurlAttrr, ...attrs } = getAttributesFromNode(node);

  ({
    className,
    unfurl: unfurlAttrr,
    ...attrs
  } = { className: "name", unfurl: "unfurl", others: [1, 2, 3] });
}
"#,
            ),
        ],
    );
}

/// Complex type arguments on assignment-like right-hand sides should match the expected shell.
#[test]
fn test_format_assignment_complex_type_arguments_layout() {
    assert_format_program_reference_widths(
        r#"// mapped type argument
const emitter = createGlobalEmitter<{
  [key in Event["type"]]: Extract<Event, { type: key }>
}>()

// object type argument
const emitter2 = createGlobalEmitter<{
  longlonglonglongKey: Extract<Event, { type: key }>
}>()

// reference type argument
// nested generic call with object-like type arguments
export class Test {
  	readonly coordinates = model.required<
		  Immutable<{
		  	latitude: number;
		  	longitude: number;
		  }>
	  >();
}

// Non-complex type arguments, as none of the type arguments of Record is a complex type.
const result = configurationService.getValue<Record<string, boolean>>(
  enalementSetting
);
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"// mapped type argument
const emitter = createGlobalEmitter<{
  [key in Event["type"]]: Extract<Event, { type: key }>;
}>();

// object type argument
const emitter2 = createGlobalEmitter<{
  longlonglonglongKey: Extract<Event, { type: key }>;
}>();

// reference type argument
// nested generic call with object-like type arguments
export class Test {
  readonly coordinates = model.required<
    Immutable<{
      latitude: number;
      longitude: number;
    }>
  >();
}

// Non-complex type arguments, as none of the type arguments of Record is a complex type.
const result =
  configurationService.getValue<Record<string, boolean>>(enalementSetting);
"#,
            ),
            (
                100,
                r#"// mapped type argument
const emitter = createGlobalEmitter<{
  [key in Event["type"]]: Extract<Event, { type: key }>;
}>();

// object type argument
const emitter2 = createGlobalEmitter<{
  longlonglonglongKey: Extract<Event, { type: key }>;
}>();

// reference type argument
// nested generic call with object-like type arguments
export class Test {
  readonly coordinates = model.required<
    Immutable<{
      latitude: number;
      longitude: number;
    }>
  >();
}

// Non-complex type arguments, as none of the type arguments of Record is a complex type.
const result = configurationService.getValue<Record<string, boolean>>(enalementSetting);
"#,
            ),
        ],
    );
}

/// Assignment-like shells with long generic calls should follow the reference width behavior.
#[test]
fn test_format_assignment_generic_call_width_behavior() {
    assert_format_program_reference_widths(
        r#"const fooRef =
        useRef<Record<string, LazyFooThingFD<T, TError> | null | undefined>>(cache);

class A {
    readonly customHeaderTemplate =
      viewChild.required<TemplateRef<{ total: number }>>("customHeader");
}

const requestTrie =
  TernarySearchTree.forPaths<IRecursiveWatchRequest>(!isLinux);
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"const fooRef =
  useRef<Record<string, LazyFooThingFD<T, TError> | null | undefined>>(cache);

class A {
  readonly customHeaderTemplate =
    viewChild.required<TemplateRef<{ total: number }>>("customHeader");
}

const requestTrie =
  TernarySearchTree.forPaths<IRecursiveWatchRequest>(!isLinux);
"#,
            ),
            (
                100,
                r#"const fooRef = useRef<Record<string, LazyFooThingFD<T, TError> | null | undefined>>(cache);

class A {
  readonly customHeaderTemplate =
    viewChild.required<TemplateRef<{ total: number }>>("customHeader");
}

const requestTrie = TernarySearchTree.forPaths<IRecursiveWatchRequest>(!isLinux);
"#,
            ),
        ],
    );
}

/// Assignment expressions should break after `=` for poorly breakable call chains.
#[test]
fn test_format_assignment_expression_call_chain_breaks_after_operator() {
    assert_format_program_reference_widths(
        r#"result = api.namespace.member().tail()
"#,
        FileType::TypeScript,
        &[(
            20,
            r#"result =
  api.namespace
    .member()
    .tail();
"#,
        )],
    );
}

/// Long left-hand sides with string rhs values should break after `=`.
#[test]
fn test_format_assignment_string_rhs_breaks_after_operator() {
    assert_format_program_reference_widths(
        r#"const veryLongVariableName = "value"
veryLongVariableName = "value"
"#,
        FileType::TypeScript,
        &[(
            20,
            r#"const veryLongVariableName =
  "value";
veryLongVariableName =
  "value";
"#,
        )],
    );
}
