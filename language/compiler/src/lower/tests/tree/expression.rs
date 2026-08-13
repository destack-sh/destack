use crate::tests::TestSession;

#[test]
fn test_lower_a_tree_fragment_with_struct_components() {
    let session = TestSession::single(
        r#"
import { TreeBuilder } from "destack:tree";

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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Panel {
    width: int32;
}

@copy
type Badge {
    label: int32;
}

function test.main.render(): Panel {
entry:
    v0: int32 = 7
    v1: Badge = aggregate (v0)
    v2: (Badge) = aggregate (v1)
    v3: Panel = call test.main.fragment<type (Badge)>(v2): ((Badge)) => Panel
    return v3
}

function test.main.fragment<type (Badge)>(v0: (Badge)): Panel {
entry(v0: (Badge)):
    v1: int32 = 1
    v2: Panel = aggregate (v1)
    return v2
}
/// @layout.struct name=Panel size=4 align=4
/// @layout.field owner=Panel index=0 name=width offset=0 size=4 align=4
/// @layout.struct name=Badge size=4 align=4
/// @layout.field owner=Badge index=0 name=label offset=0 size=4 align=4
/// @layout.tuple name=type@8 size=4 align=4
/// @layout.element owner=type@8 index=0 offset=0 size=4 align=4
"#,
    );
}
