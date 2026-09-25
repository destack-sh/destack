use tspp_dir as dir;

use crate::tests::TestMatcher;

/// Match dependency item names and aliases independently of their declaration.
#[test]
fn test_match_dependency_item() {
    TestMatcher::context(
        r#"import { $NAME as $ALIAS } from "module";"#,
        dir::NodeType::DependencyItem,
        r#"
import { first as primary, second, third as tertiary } from "module";
"#,
    )
    .assert(
        r#"
import { first as primary, second, third as tertiary } from "module";
         ^^^^^^^^^^^^^^^^ match NAME.name="first" ALIAS.name="primary"
                                   ^^^^^^^^^^^^^^^^^ match NAME.name="third" ALIAS.name="tertiary"
"#,
    );
}
