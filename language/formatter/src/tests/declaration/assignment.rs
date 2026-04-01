use crate::{assert_format_program_idempotent, assert_format_program_reference_widths};
use destack_source::FileType;

/// Type alias comments after `=` should stay stable across passes.
#[test]
fn test_format_typescript_union_head_comment_after_equals_is_idempotent() {
    assert_format_program_idempotent!(
        r#"type Aa1 = /*1*/ | /*2*/ C | D;
"#,
        FileType::TypeScript
    );
}

/// Assignment comment seams should stay attached to the formatted assignment shell.
#[test]
fn test_format_typescript_assignment_comment_seams() {
    assert_format_program_reference_widths(
        r#"var longlonglonglonglonglong = /*#__PURE__*/_interopDefaultLegacy(aaaaaaaaaaaaaaa);
var short = /*#__PURE__*/_interopDefaultLegacy(b);

const jestPackageJson =
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  require(jestPath);

{
  sourcemap =
  /** @type {'inline' | 'hidden' | 'sourcemap'} */ (
      process.env.WORKER_MODE
    ) || sourcemap;
}

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

{
  sourcemap =
    /** @type {'inline' | 'hidden' | 'sourcemap'} */ (
      process.env.WORKER_MODE
    ) || sourcemap;
}

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

{
  sourcemap =
    /** @type {'inline' | 'hidden' | 'sourcemap'} */ (process.env.WORKER_MODE) || sourcemap;
}

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

/// Call-expression type-argument seams should preserve leading comments.
#[test]
fn test_format_typescript_assignment_call_with_type_args_comment_seams() {
    assert_format_program_reference_widths(
        r#"// Type arguments with speculative formatting should not lose comments
export const globalRegistry: $ZodRegistry = /*@__PURE__*/ registry();

// Comments before call expressions with type arguments should be preserved
const r = /* THIS */ f<Type>()
const s = /* comment */ foo<A | B | C>()
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"// Type arguments with speculative formatting should not lose comments
export const globalRegistry: $ZodRegistry = /*@__PURE__*/ registry();

// Comments before call expressions with type arguments should be preserved
const r = /* THIS */ f<Type>();
const s = /* comment */ foo<A | B | C>();
"#,
            ),
            (
                100,
                r#"// Type arguments with speculative formatting should not lose comments
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
fn test_format_typescript_type_alias_conditional_layout() {
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
fn test_format_typescript_assignment_arrow_function_layout() {
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

/// Complex destructuring assignments should use the reference break-left-hand-side layout.
#[test]
fn test_format_typescript_assignment_break_left_hand_side_layout() {
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
fn test_format_typescript_assignment_complex_type_arguments_layout() {
    assert_format_program_reference_widths(
        r#"// Type argument is a `TSMappedType`
const emitter = createGlobalEmitter<{
  [key in Event["type"]]: Extract<Event, { type: key }>
}>()

// Type argument is a `TSTypeLiteral`
const emitter2 = createGlobalEmitter<{
  longlonglonglongKey: Extract<Event, { type: key }>
}>()

// Type argument is a `TSTypeReference`
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
                r#"// Type argument is a `TSMappedType`
const emitter = createGlobalEmitter<{
  [key in Event["type"]]: Extract<Event, { type: key }>;
}>();

// Type argument is a `TSTypeLiteral`
const emitter2 = createGlobalEmitter<{
  longlonglonglongKey: Extract<Event, { type: key }>;
}>();

// Type argument is a `TSTypeReference`
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
                r#"// Type argument is a `TSMappedType`
const emitter = createGlobalEmitter<{
  [key in Event["type"]]: Extract<Event, { type: key }>;
}>();

// Type argument is a `TSTypeLiteral`
const emitter2 = createGlobalEmitter<{
  longlonglonglongKey: Extract<Event, { type: key }>;
}>();

// Type argument is a `TSTypeReference`
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
fn test_format_typescript_assignment_generic_call_width_behavior() {
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
