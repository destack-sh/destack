use tspp_source::DiagnosticSeverity;

use super::{Example, ExampleFile, Expectation, Probe};

/// Read the files, cut, and probes of one section into one example with annotation-free sources.
#[test]
fn test_parse_document_examples() {
    let document = r#"# Structs

## Copies

```tspp src/geometry.tspp
export struct Point { x: float64; y: float64; }
```

```tspp src/main.tspp
import { Point } from "./geometry.tspp";
// ---cut---
let pivot = Point { x: 1.0, y: 1.0 };
pivot
//^? Point
pivot.x // => 1
const b: uint8 = pivot.x as uint8;
//               ^^^^^^^^^^^^^^^^ error[invalid-cast]: type 'float64' cannot be cast to 'uint8'
```

```ts
ignored();
```
"#;
    let examples = Example::parse_document(document).expect("parse examples");

    // expect one example with annotations turned into probes on clean sources
    let geometry = "export struct Point { x: float64; y: float64; }\n";
    let main = "import { Point } from \"./geometry.tspp\";\nlet pivot = Point { x: 1.0, y: 1.0 };\npivot\npivot.x\nconst b: uint8 = pivot.x as uint8;\n";
    let geometry_start = document.find("export struct").unwrap();
    let main_start = document.find("import { Point }").unwrap();
    let main_end = document.find("```\n\n```ts\n").unwrap();
    assert_eq!(
        examples,
        vec![Example {
            name: "Structs > Copies".to_string(),
            files: vec![
                ExampleFile {
                    path: "src/geometry.tspp".to_string(),
                    source: geometry.to_string(),
                    shown: 0,
                    probes: Vec::new(),
                    block: geometry_start..geometry_start + geometry.len(),
                },
                ExampleFile {
                    path: "src/main.tspp".to_string(),
                    source: main.to_string(),
                    shown: 41,
                    probes: vec![
                        Probe {
                            range: 81..81,
                            expectation: Expectation::Type("Point".to_string()),
                        },
                        Probe {
                            range: 85..92,
                            expectation: Expectation::Value("1".to_string()),
                        },
                        Probe {
                            range: 110..126,
                            expectation: Expectation::Diagnostic {
                                severity: DiagnosticSeverity::Error,
                                code: "invalid-cast".to_string(),
                                message: "type 'float64' cannot be cast to 'uint8'".to_string(),
                            },
                        },
                    ],
                    block: main_start..main_end,
                },
            ],
        }]
    );
}
