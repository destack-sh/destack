use crate::tests::{DirRows, TestSession};

#[test]
fn test_reject_cross_module_duplicate_single_slot_member() {
    let session = TestSession::builder()
        .module(
            "widget.ds",
            r#"
export struct Widget {}

export extension of Widget {
    get size(): usize {
        return 1;
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Widget } from "./widget.ds";

extension of Widget {
    get size(): usize {
        return 2;
    }
}
"#,
        )
        .build();

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
import { Widget } from "./widget.ds";

extension of Widget {
    get size(): usize {
        return 2;
    }
}

=== checked ===
import { Widget } from "./widget.ds";

extension of Widget {
    get size(): usize {
        return 2;
    }
}
"#,
        r#"
/// @diagnostic.error id=duplicate-member message="member 'size' is already declared for 'Widget' by another visible extension"
/// @diagnostic.label line=5 column=9 span="size" line_source="get size(): usize {"
"#,
    );
}

#[test]
fn test_allow_same_module_repeated_single_slot_member() {
    let session = TestSession::single(
        r#"
struct Widget {}

extension of Widget {
    get size(): usize {
        return 1;
    }
}

extension of Widget {
    get size(): usize {
        return 2;
    }
}

declare const widget: Widget;
const size = widget.size;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
struct Widget {}

extension of Widget {
    get size(): usize {
        return 1;
    }
}

extension of Widget {
    get size(): usize {
        return 2;
    }
}

declare const widget: Widget;
const size: usize = widget.size;

=== checked ===
struct Widget {}

extension of Widget {
    get size(): usize {
        return 1;
    }
}

extension of Widget {
    get size(): usize {
        return 2;
    }
}

declare const widget: Widget;
const size = widget.size;
"#,
        r#"

"#,
    );
}
