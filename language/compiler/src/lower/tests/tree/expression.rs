use crate::tests::TestSession;

#[test]
fn test_lower_a_tree_fragment_with_struct_components() {
    let session = TestSession::single(
        r#"
import { TreeBuilder } from "tspp:tree";

struct Panel {
    width: int32;
}

extension of Panel implements TreeBuilder {
    type Tags = {};

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return Panel { width: 0 };
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return Panel { width: 1 };
    }
}

struct Badge {
    label: int32;
}

function render(): Panel {
    const page: Panel = <><Badge label={7}/></>;
    return page;
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.render",
        r#"
type test.main.Panel {
    width: int32;
}

type test.main.Badge {
    label: int32;
}

function test.main.render(): test.main.Panel {
    local l0: test.main.Panel

entry:
    v0: int32 = 7
    v1: test.main.Badge = aggregate (v0)
    v2: (test.main.Badge) = aggregate (v1)
    v3: test.main.Panel = call test.main.Panel.TreeBuilder.fragment<(test.main.Badge)>(v2): ((test.main.Badge)) => test.main.Panel
    store l0, v3
    v4: test.main.Panel = load l0
    return v4
}

/// @layout.struct name=test.main.Panel size=4 align=4
/// @layout.field owner=test.main.Panel index=0 name=width offset=0 size=4 align=4
/// @layout.struct name=test.main.Badge size=4 align=4
/// @layout.field owner=test.main.Badge index=0 name=label offset=0 size=4 align=4
/// @layout.tuple name=type@28 size=4 align=4
/// @layout.element owner=type@28 index=0 offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.Panel.TreeBuilder.fragment<(test.main.Badge)>",
        r#"
type test.main.Panel {
    width: int32;
}

type test.main.Badge {
    label: int32;
}

shared function test.main.Panel.TreeBuilder.fragment<(test.main.Badge)>(v0: (test.main.Badge)): test.main.Panel;

/// @layout.struct name=test.main.Panel size=4 align=4
/// @layout.field owner=test.main.Panel index=0 name=width offset=0 size=4 align=4
/// @layout.struct name=test.main.Badge size=4 align=4
/// @layout.field owner=test.main.Badge index=0 name=label offset=0 size=4 align=4
/// @layout.tuple name=type@28 size=4 align=4
/// @layout.element owner=type@28 index=0 offset=0 size=4 align=4
"#,
    );
}
