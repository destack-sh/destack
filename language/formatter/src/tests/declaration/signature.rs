use crate::{
    TsppFormatOptions, assert_format, assert_format_program_idempotent,
    assert_format_program_reference_widths, assert_format_program_roundtrip_with_file_type,
};
use tspp_source::FileType;

#[test]
fn test_format_parameter() {
    assert_format!(
        "x: int32",
        "x: int32",
        |p| p.parse_parameter_fragment(),
        TsppFormatOptions::default()
    );
}

/// Malformed parameters should preserve their authored source.
#[test]
fn test_format_recovered_parameter() {
    assert_format!(
        "(x, =, y)",
        "=",
        |parser| {
            let parameters = parser.parse_parameter_list_fragment()?;
            Ok(parameters[1])
        },
        TsppFormatOptions::default()
    );
}

#[test]
fn test_format_float_parameter_types() {
    assert_format_program_roundtrip_with_file_type(
        r#"function convert(a: float32, b: float64): void {}
"#,
        r#"function convert(a: float32, b: float64): void {}
"#,
        FileType::Tspp,
        TsppFormatOptions::default(),
    );
}

#[test]
fn test_format_parameter_with_default() {
    assert_format!(
        "x: int32 = 1",
        "x: int32 = 1",
        |p| p.parse_parameter_fragment(),
        TsppFormatOptions::default()
    );
}

#[test]
fn test_format_receiver_shorthand() {
    assert_format_program_roundtrip_with_file_type(
        r#"type PlainVisitor = (this, value: Node) => void
type Visitor = (readonly this, value: Node) => void
type BorrowingVisitor = (&readonly this, value: Node) => void
extension of Buffer {
push(&this, value: uint8): void { undefined! }
}
"#,
        r#"type PlainVisitor = (this, value: Node) => void;
type Visitor = (readonly this, value: Node) => void;
type BorrowingVisitor = (&readonly this, value: Node) => void;
extension of Buffer {
    push(&this, value: uint8): void {
        undefined!;
    }
}
"#,
        FileType::Tspp,
        TsppFormatOptions::default(),
    );
}

#[test]
fn test_format_variadic_generic_parameters() {
    assert_format_program_roundtrip_with_file_type(
        r#"type Callback<...Parameters, Return> = (...parameters: Parameters) => Return
function tensor<const ...Shape: readonly usize[]>(value: Tensor<...Shape>): void {}
"#,
        r#"type Callback<...Parameters, Return> = (...parameters: Parameters) => Return;
function tensor<const ...Shape: readonly usize[]>(value: Tensor<...Shape>): void {}
"#,
        FileType::Tspp,
        TsppFormatOptions::default(),
    );
}

/// Defaulted pattern parameters should keep comments inside the pattern.
#[test]
fn test_format_pattern_parameter_default_comments() {
    assert_format_program_roundtrip_with_file_type(
        r#"({
  // comment1
  random = Math.random,
  // comment2
  sqrt = Math.sqrt,
} = {}) => {};
"#,
        r#"({
  // comment1
  random = Math.random,
  // comment2
  sqrt = Math.sqrt,
} = {}) => {};
"#,
        FileType::Tspp,
        TsppFormatOptions::default().with_indent_width(2),
    );
}

/// Generic parameter constraints should print as colon bounds.
#[test]
fn test_format_generic_parameter_colon_bounds() {
    assert_format_program_roundtrip_with_file_type(
        r#"type Value<T: string> = T
"#,
        r#"type Value<T: string> = T;
"#,
        FileType::Tspp,
        TsppFormatOptions::default(),
    );
}

/// Trailing separator line comments in parameter lists should stay idempotent.
#[test]
fn test_format_signature_trailing_separator_line_comment_is_idempotent() {
    assert_format_program_idempotent!(
        r#"f2 = (
  currentRequest: {a: number},
  // this deliberately long comment keeps the separator attachment in the broken layout
): number => {};
"#,
        FileType::Tspp
    );
}

/// Own-line block separator comments in parameter lists should stay idempotent.
#[test]
fn test_format_signature_trailing_separator_block_comment_is_idempotent() {
    assert_format_program_idempotent!(
        r#"let x = {
  getSectionMode(
    pageMetaData: PageMetaData,
    sectionMetaData: SectionMetaData
    /* $FlowFixMe This error was exposed while converting keyMirror
     * to keyMirrorRecursive */
  ): $Enum<SectionMode> {
  }
}

class X2 {
  getSectionMode(
    pageMetaData: PageMetaData,
    sectionMetaData: SectionMetaData = ["unknown"]
    /* $FlowFixMe This error was exposed while converting keyMirror
     * to keyMirrorRecursive */
  ): $Enum<SectionMode> {
  }
}
"#,
        FileType::Tspp
    );
}

/// Method comments should stay attached to the final method boundary.
#[test]
fn test_format_method_comments() {
    assert_format_program_reference_widths(
        r#"class A {
  m1(element: Element, key: string, missing: undefined) /* block comment */ {
    // method body
  }
  m2(element: Element, key: string, missing: undefined): void /* block comment */ {
    // method body
  }
  m3(tagName: string, rect: number[]): void // line comment
  {
    // method body
  }
  m4(tagName: string, rect: number[]) // line comment
  {
    // method body
  }
}
"#,
        FileType::Tspp,
        &[
            (
                80,
                r#"class A {
  m1(element: Element, key: string, missing: undefined) /* block comment */ {
    // method body
  }
  m2(
    element: Element,
    key: string,
    missing: undefined,
  ): void /* block comment */ {
    // method body
  }
  m3(tagName: string, rect: number[]): void {
    // line comment
    // method body
  }
  m4(tagName: string, rect: number[]) {
    // line comment
    // method body
  }
}
"#,
            ),
            (
                100,
                r#"class A {
  m1(element: Element, key: string, missing: undefined) /* block comment */ {
    // method body
  }
  m2(element: Element, key: string, missing: undefined): void /* block comment */ {
    // method body
  }
  m3(tagName: string, rect: number[]): void {
    // line comment
    // method body
  }
  m4(tagName: string, rect: number[]) {
    // line comment
    // method body
  }
}
"#,
            ),
        ],
    );
}

/// Type-literal parameter defaults should keep the expected signature layout.
#[test]
fn test_format_type_literal_parameter_layout() {
    assert_format_program_reference_widths(
        r#"export default function useTagsCount({
  query,
}: { query?: Record<unknown, unknown> } = {}) {
}
"#,
        FileType::Tspp,
        &[
            (
                80,
                r#"export default function useTagsCount({
  query,
}: { query?: Record<unknown, unknown> } = {}) {}
"#,
            ),
            (
                100,
                r#"export default function useTagsCount({ query }: { query?: Record<unknown, unknown> } = {}) {}
"#,
            ),
        ],
    );
}

/// Mapped types should keep canonical bracket spacing.
#[test]
fn test_format_mapped_type_bracket_spacing() {
    let input = r#"export type Bar<T> = {[P in keyof T]: string}
"#;

    assert_format_program_roundtrip_with_file_type(
        input,
        r#"export type Bar<T> = { [P in keyof T]: string };
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(80).with_indent_width(2),
    );

    assert_format_program_roundtrip_with_file_type(
        input,
        r#"export type Bar<T> = { [P in keyof T]: string };
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(100).with_indent_width(2),
    );
}

/// Parameter type comments should stay on the type side of the separator.
#[test]
fn test_format_parameter_name_type_comments() {
    assert_format_program_reference_widths(
        r#"// comment between parameter name and type annotation
function f(x /* a */ : number) {}

// Additional test cases
function g(y /* comment */ : string, z /* another */ : boolean) {}

// With different comment styles
function h(a /* inline */ : number) {}

// Multiple parameters with comments
const arrow = (x /* c1 */ : number, y /* c2 */ : string) => {};

// Optional parameters with comments
function optional(x? /* comment */ : number) {}
function optionalMultiple(a? /* c1 */ : string, b? /* c2 */ : number) {}
"#,
        FileType::Tspp,
        &[
            (
                80,
                r#"// comment between parameter name and type annotation
function f(x /* a */ : number) {}

// Additional test cases
function g(y /* comment */ : string, z /* another */ : boolean) {}

// With different comment styles
function h(a /* inline */ : number) {}

// Multiple parameters with comments
const arrow = (x /* c1 */ : number, y /* c2 */ : string) => {};

// Optional parameters with comments
function optional(x? /* comment */ : number) {}
function optionalMultiple(a? /* c1 */ : string, b? /* c2 */ : number) {}
"#,
            ),
            (
                100,
                r#"// comment between parameter name and type annotation
function f(x /* a */ : number) {}

// Additional test cases
function g(y /* comment */ : string, z /* another */ : boolean) {}

// With different comment styles
function h(a /* inline */ : number) {}

// Multiple parameters with comments
const arrow = (x /* c1 */ : number, y /* c2 */ : string) => {};

// Optional parameters with comments
function optional(x? /* comment */ : number) {}
function optionalMultiple(a? /* c1 */ : string, b? /* c2 */ : number) {}
"#,
            ),
        ],
    );
}

/// Interface method parameter separator comments should stay with the same parameter.
#[test]
fn test_format_interface_method_parameter_separator_comment() {
    assert_format_program_roundtrip_with_file_type(
        r#"interface Worker {
  run(
    value: string, // value-tail
  ): number
}
"#,
        r#"interface Worker {
    run(
        value: string, // value-tail
    ): number;
}
"#,
        FileType::Tspp,
        TsppFormatOptions::default(),
    );
}

/// Signature return separator comments should stay attached to the return type.
#[test]
fn test_format_signature_return_separator_comment() {
    assert_format_program_roundtrip_with_file_type(
        r#"interface Worker {
  run(): // return-tail
  Promise<void>
}
"#,
        r#"interface Worker {
    run(): // return-tail
    Promise<void>;
}
"#,
        FileType::Tspp,
        TsppFormatOptions::default(),
    );
}

/// Method-style signatures should keep trailing generic commas in broken layouts.
#[test]
fn test_format_grouped_method_signature_layout() {
    assert_format_program_reference_widths(
        r#"type A = {
  new(...args): T<{
    A
  }
  >
};


type A1 = {
  (...args): T<{
    A
  }
  >
};

type A2 = {
  bar(
    ...args
  ): T<{
    A
  }>
}

class A3 {
  constructor(
    eventName: string,
    data?: object,
  ) { }
}

class A4 {
  publicLog<
    E: ClassifiedEvent<OmitMetadata<T>>,
    T: IGDPRProperty,
  >(
    eventName: string, data?: object
  ) {
  }
}

const A5 = {
  publicLog<
    E: ClassifiedEvent<OmitMetadata<T>>,
    T: IGDPRProperty,
  >(
    eventName: string,
    data?: object,
  ) { }
}
"#,
        FileType::Tspp,
        &[
            (
                80,
                r#"type A = {
  new (...args): T<{
    A;
  }>;
};

type A1 = {
  (...args): T<{
    A;
  }>;
};

type A2 = {
  bar(...args): T<{
    A;
  }>;
};

class A3 {
  constructor(eventName: string, data?: object) {}
}

class A4 {
  publicLog<E: ClassifiedEvent<OmitMetadata<T>>, T: IGDPRProperty>(
    eventName: string,
    data?: object,
  ) {}
}

const A5 = {
  publicLog<E: ClassifiedEvent<OmitMetadata<T>>, T: IGDPRProperty>(
    eventName: string,
    data?: object,
  ) {},
};
"#,
            ),
            (
                100,
                r#"type A = {
  new (...args): T<{
    A;
  }>;
};

type A1 = {
  (...args): T<{
    A;
  }>;
};

type A2 = {
  bar(...args): T<{
    A;
  }>;
};

class A3 {
  constructor(eventName: string, data?: object) {}
}

class A4 {
  publicLog<E: ClassifiedEvent<OmitMetadata<T>>, T: IGDPRProperty>(
    eventName: string,
    data?: object,
  ) {}
}

const A5 = {
  publicLog<E: ClassifiedEvent<OmitMetadata<T>>, T: IGDPRProperty>(
    eventName: string,
    data?: object,
  ) {},
};
"#,
            ),
        ],
    );
}

/// Parameter layout should keep the expected hugging and object-pattern behavior.
#[test]
fn test_format_parameter_layout() {
    assert_format_program_reference_widths(
        r#" const assertFilteringFor = (expected: {
   [T in TestFilterTerm]?: boolean;
 }) => {};

// Should not hug
export async function update(
  options: {
    eventKey?: ConfigEvent.ConfigEventKey;
  } = {},
): Promise<void> {
}

(
  options,
  { log, logger, messenger }: {
    log: LogFun;
    logger: Logger;
    messenger: Messenger;
  }) => {

}

export function useCopyToClipboard({ timeout = 2000, onCopy }: {
  timeout?: number;
  onCopy?: () => void;
} = {}) { }

function callbackUrl(
  { baseUrl, params }: { baseUrl: string; params?: string } = {
    baseUrl: "",
    params: undefined,
  }
) { }

function parseTitle(
  item: PageObjectResponse | DatabaseObjectResponse,
  {
    maxLength = DocumentValidation.maxTitleLength,
  }: { maxLength?: number } = {}
) {}
"#,
        FileType::Tspp,
        &[
            (
                80,
                r#"const assertFilteringFor = (expected: {
  [T in TestFilterTerm]?: boolean;
}) => {};

// Should not hug
export async function update(
  options: {
    eventKey?: ConfigEvent.ConfigEventKey;
  } = {},
): Promise<void> {}

(
  options,
  {
    log,
    logger,
    messenger,
  }: {
    log: LogFun;
    logger: Logger;
    messenger: Messenger;
  },
) => {};

export function useCopyToClipboard({
  timeout = 2000,
  onCopy,
}: {
  timeout?: number;
  onCopy?: () => void;
} = {}) {}

function callbackUrl(
  { baseUrl, params }: { baseUrl: string; params?: string } = {
    baseUrl: "",
    params: undefined,
  },
) {}

function parseTitle(
  item: PageObjectResponse | DatabaseObjectResponse,
  {
    maxLength = DocumentValidation.maxTitleLength,
  }: { maxLength?: number } = {},
) {}
"#,
            ),
            (
                100,
                r#"const assertFilteringFor = (expected: {
  [T in TestFilterTerm]?: boolean;
}) => {};

// Should not hug
export async function update(
  options: {
    eventKey?: ConfigEvent.ConfigEventKey;
  } = {},
): Promise<void> {}

(
  options,
  {
    log,
    logger,
    messenger,
  }: {
    log: LogFun;
    logger: Logger;
    messenger: Messenger;
  },
) => {};

export function useCopyToClipboard({
  timeout = 2000,
  onCopy,
}: {
  timeout?: number;
  onCopy?: () => void;
} = {}) {}

function callbackUrl(
  { baseUrl, params }: { baseUrl: string; params?: string } = {
    baseUrl: "",
    params: undefined,
  },
) {}

function parseTitle(
  item: PageObjectResponse | DatabaseObjectResponse,
  { maxLength = DocumentValidation.maxTitleLength }: { maxLength?: number } = {},
) {}
"#,
            ),
        ],
    );
}

/// Rest parameter type queries should follow the shared parameter layout.
#[test]
fn test_format_parameter_type_query_layout() {
    assert_format_program_reference_widths(
        r#"useStableCallback(function useShowToast(
    ...args: Parameters<typeof toastService.addToastItem>
  ): void {
    toastService.addToastItem(...args);
  });
"#,
        FileType::Tspp,
        &[
            (
                80,
                r#"useStableCallback(function useShowToast(
  ...args: Parameters<typeof toastService.addToastItem>
): void {
  toastService.addToastItem(...args);
});
"#,
            ),
            (
                100,
                r#"useStableCallback(function useShowToast(
  ...args: Parameters<typeof toastService.addToastItem>
): void {
  toastService.addToastItem(...args);
});
"#,
            ),
        ],
    );
}

#[test]
fn test_format_interface_default_method_body_roundtrip() {
    assert_format_program_roundtrip_with_file_type(
        r#"newtype interface Error {
    source(): Dynamic<Error> | undefined {
        undefined
    }
}
"#,
        r#"newtype interface Error {
    source(): Dynamic<Error> | undefined {
        undefined
    }
}
"#,
        FileType::Tspp,
        TsppFormatOptions::default(),
    );
}
