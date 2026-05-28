use crate::tests::{DirRows, TestSession};

#[test]
fn test_satisfies_preserves_object_literal_members() {
    let session = TestSession::single(
        r#"
type Mode = "dev" | "prod";
type Shape = { mode: Mode };

let config = { mode: "dev" } satisfies Shape;
const mode = config.mode;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
type Mode = "dev" | "prod";
/// @type.symbol symbol=Mode type="dev" | "prod"

type Shape = { mode: Mode };
/// @type.symbol symbol=Shape type={ mode: "dev" | "prod" }
/// @type.symbol symbol=Shape.mode type="dev" | "prod"

let config = { mode: "dev" } satisfies Shape;
/// @type.symbol symbol=config type={ mode: "dev" }
/// @type.node source="{ mode: \"dev\" } satisfies Shape" type={ mode: "dev" }
/// @type.node source="{ mode: \"dev\" }" type={ mode: "dev" }
/// @type.node source="\"dev\"" type="dev"

const mode = config.mode;
/// @type.symbol symbol=mode type="dev"
/// @type.node source=config type={ mode: "dev" }
/// @type.node source=config.mode type="dev"
/// @resolution.name source=config target=config
/// @resolution.member source=config.mode receiver={ mode: "dev" } kind=field key=mode
"#,
    );
}
