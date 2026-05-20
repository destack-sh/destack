use crate::tests::{DirRows, TestSession};

#[test]
fn test_bind_global_scope() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
import { Process, Task } from "runtime";

global {
    let process: Process;

    function schedule(task: Task) {
        task;
    }
}

let process: string = "local";
"#,
        )
        .build();

    compiler.assert_dir_bound("main.ds", DirRows::binding().with_summaries(), r#"
import { Process, Task } from "runtime";
/// @binding.symbol symbol=Process role=local form=import scope=<module>@1
/// @binding.symbol symbol=Task role=local form=import scope=<module>@2

global {
/// @binding.scope scope=scope1 kind=global parent=<module>@3

    let process: Process;
    /// @binding.symbol symbol=process#1 role=local form=variable scope=scope1@0 mutability=mutable origin=global

    function schedule(task: Task) {
    /// @binding.symbol symbol=schedule role=item form=function scope=scope1@1 origin=global
    /// @binding.scope scope=schedule kind=function parent=scope1@2 owner=schedule
    /// @binding.symbol symbol=task role=local form=variable scope=schedule@0
    /// @binding.scope scope=scope3 kind=block parent=schedule@1

        task;
    }
}

let process: string = "local";
/// @binding.symbol symbol=process#2 role=local form=variable scope=<module>@3 mutability=mutable
/// @binding.symbol symbol=<module> role=namespace form=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>

/// @binding.summary symbols=7 scopes=4 declarations=6 node_scopes=21
"#,
    );
}
