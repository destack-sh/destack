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

    session.assert_dir_and_diagnostics(
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

=== dir ===
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

    session.assert_dir_and_diagnostics(
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

=== dir ===
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

#[test]
fn test_reject_an_unnamed_exported_extension_of_a_nonlocal_type() {
    let session = TestSession::builder()
        .module(
            "widget.ds",
            r#"
export struct Widget {}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Widget } from "./widget.ds";

export extension of Widget {
    get size(): usize {
        return 1;
    }
}
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
import { Widget } from "./widget.ds";

export extension of Widget {
    get size(): usize {
        return 1;
    }
}

=== dir ===
import { Widget } from "./widget.ds";

export extension of Widget {
    get size(): usize {
        return 1;
    }
}
"#,
        r#"
/// @diagnostic.error id=unnamed-exported-nonlocal-extension message="exported extension on nonlocal type 'Widget' must have a name"
/// @diagnostic.label line=4 column=21 span="Widget" line_source="export extension of Widget {"
"#,
    );
}
