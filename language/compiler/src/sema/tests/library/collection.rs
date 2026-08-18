use crate::tests::TestSession;

/// Resolve scalar and borrowed string membership through their matching protocols.
#[test]
fn test_resolve_collection_membership() {
    let session = TestSession::single(
        r#"
import { Array, Map, Set } from "destack:collections";
import { StringSlice } from "destack:string";

declare const integers: Array<int32>;
declare const integer: int32;
declare const floats: Array<float64>;
declare const float: float64;
declare const strings: Array<string>;
declare const stringValue: string;
declare const stringSlice: &readonly StringSlice;
declare const stringMap: Map<string, int32>;
declare const stringSet: Set<string>;

integers.includes(integer);
floats.includes(float);
strings.includes(stringValue);
strings.includes(stringSlice);
stringMap.has(stringSlice);
stringSet.has(stringSlice);
"#,
    );

    session.assert_dir_diagnostics("main.ds", "");
}
