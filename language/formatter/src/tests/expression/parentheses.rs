use crate::assert_format_program_reference_widths;
use destack_source::FileType;

/// Type-cast comment wrappers should match the reference shell exactly.
#[test]
fn test_format_type_cast_comment_node_fixture() {
    assert_format_program_reference_widths(
        r#"!left &&
/** @type {boolean} */
(
  /** @type {Identifier} */
  (a) === "call" ||
    /** @type {Identifier} */
    (b) === "bind"
//  ^^^^^^^^^^^^^^ No need to wrap with parentheses here because the type cast node is already wrapped with parentheses.
) && right;

/** @type {Number} */ (a + b)();
"#,
        FileType::JavaScript,
        &[
            (
                80,
                r#"!left &&
  /** @type {boolean} */
  (
    /** @type {Identifier} */
    (a) === "call" ||
      /** @type {Identifier} */
      (b) === "bind"
    //  ^^^^^^^^^^^^^^ No need to wrap with parentheses here because the type cast node is already wrapped with parentheses.
  ) &&
  right;

/** @type {Number} */ (a + b)();
"#,
            ),
            (
                100,
                r#"!left &&
  /** @type {boolean} */
  (
    /** @type {Identifier} */
    (a) === "call" ||
      /** @type {Identifier} */
      (b) === "bind"
    //  ^^^^^^^^^^^^^^ No need to wrap with parentheses here because the type cast node is already wrapped with parentheses.
  ) &&
  right;

/** @type {Number} */ (a + b)();
"#,
            ),
        ],
    );
}
