use crate::tests::{DirRows, TestSession};

#[test]
fn test_conform_through_reexported_interface_symbol() {
    let session = TestSession::builder()
        .module(
            "shape.ds",
            r#"
export newtype interface Sized {
    size(): usize;
}
"#,
        )
        .module(
            "reexport.ds",
            r#"
export { Sized } from "./shape.ds";
"#,
        )
        .module(
            "main.ds",
            r#"
import { Sized } from "./reexport.ds";

struct Box implements Sized {
    size(): usize {
        return 1;
    }
}

declare const box: Box;
const sized: Sized = box;
"#,
        )
        .build();

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
import { Sized } from "./reexport.ds";

struct Box implements Sized {
    size(): usize {
        return 1;
    }
}

declare const box: Box;
const sized: Dynamic<Sized> = box as Dynamic<Sized>;

=== checked ===
import { Sized } from "./reexport.ds";

struct Box implements Sized {
    size(): usize {
        return 1;
    }
}

declare const box: Box;
const sized: Sized = box;
"#,
        r#"

"#,
    );
}
