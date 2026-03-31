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
