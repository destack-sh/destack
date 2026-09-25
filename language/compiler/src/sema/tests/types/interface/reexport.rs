use crate::tests::{DirRows, TestSession};

#[test]
fn test_conform_through_reexported_interface_symbol() {
    let session = TestSession::builder()
        .module(
            "shape.tspp",
            r#"
export newtype interface Sized {
    size(): usize;
}
"#,
        )
        .module(
            "reexport.tspp",
            r#"
export { Sized } from "./shape.tspp";
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Sized } from "./reexport.tspp";

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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::none(),
        r#"
=== annotated ===
import { Sized } from "./reexport.tspp";

struct Box implements Sized {
    size(): usize {
        return 1;
    }
}

declare const box: Box;
const sized: Sized = box as Sized;

=== dir ===
import { Sized } from "./reexport.tspp";

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
