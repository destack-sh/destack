use crate::tests::{DirRows, TestSession};

#[test]
fn test_import_resolves_package_export() {
    let compiler = TestSession::new()
        .data(
            "destack.json",
            r#"
{
    "workspace": {
        "packages": ["packages/*", "packages/@*/*"]
    }
}
"#,
        )
        .data(
            "packages/app/destack.json",
            r#"
{
    "name": "app",
    "dependencies": {
        "@acme/ui": {
            "source": "workspace"
        }
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/destack.json",
            r#"
{
    "name": "@acme/ui",
    "exports": {
        "./button": {
            "kind": "module",
            "path": "button.ds"
        }
    }
}
"#,
        )
        .module(
            "packages/app/main.ds",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .module(
            "packages/@acme/ui/button.ds",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "packages/app/main.ds",
        DirRows::modules().with_summaries(),
        r#"
import { Button } from "@acme/ui/button";
/// @module.edge relation=import specifier=@acme/ui/button module=packages/@acme/ui/button.ds

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_resolves_package_root_export() {
    let compiler = TestSession::new()
        .data(
            "destack.json",
            r#"
{
    "workspace": {
        "packages": ["packages/*", "packages/@*/*"]
    }
}
"#,
        )
        .data(
            "packages/app/destack.json",
            r#"
{
    "name": "app",
    "dependencies": {
        "@acme/ui": {
            "source": "workspace"
        }
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/destack.json",
            r#"
{
    "name": "@acme/ui",
    "exports": {
        ".": {
            "kind": "module",
            "path": "index.ds"
        }
    }
}
"#,
        )
        .module(
            "packages/app/main.ds",
            r#"
import { Button } from "@acme/ui";
"#,
        )
        .module(
            "packages/@acme/ui/index.ds",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "packages/app/main.ds",
        DirRows::modules().with_summaries(),
        r#"
import { Button } from "@acme/ui";
/// @module.edge relation=import specifier=@acme/ui module=packages/@acme/ui/index.ds

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_resolves_package_pattern_export() {
    let compiler = TestSession::new()
        .data(
            "destack.json",
            r#"
{
    "workspace": {
        "packages": ["packages/*", "packages/@*/*"]
    }
}
"#,
        )
        .data(
            "packages/app/destack.json",
            r#"
{
    "name": "app",
    "dependencies": {
        "@acme/ui": {
            "source": "workspace"
        }
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/destack.json",
            r#"
{
    "name": "@acme/ui",
    "exports": {
        "./*": {
            "kind": "module",
            "path": "src/*.ds"
        }
    }
}
"#,
        )
        .module(
            "packages/app/main.ds",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .module(
            "packages/@acme/ui/src/button.ds",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "packages/app/main.ds",
        DirRows::modules().with_summaries(),
        r#"
import { Button } from "@acme/ui/button";
/// @module.edge relation=import specifier=@acme/ui/button module=packages/@acme/ui/src/button.ds

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_resolves_nested_package_pattern_export() {
    let compiler = TestSession::new()
        .data(
            "destack.json",
            r#"
{
    "workspace": {
        "packages": ["packages/*", "packages/@*/*"]
    }
}
"#,
        )
        .data(
            "packages/app/destack.json",
            r#"
{
    "name": "app",
    "dependencies": {
        "@acme/ui": {
            "source": "workspace"
        }
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/destack.json",
            r#"
{
    "name": "@acme/ui",
    "exports": {
        "./*": {
            "kind": "module",
            "path": "src/*.ds"
        }
    }
}
"#,
        )
        .module(
            "packages/app/main.ds",
            r#"
import { Button } from "@acme/ui/forms/button";
"#,
        )
        .module(
            "packages/@acme/ui/src/forms/button.ds",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "packages/app/main.ds",
        DirRows::modules().with_summaries(),
        r#"
import { Button } from "@acme/ui/forms/button";
/// @module.edge relation=import specifier=@acme/ui/forms/button module=packages/@acme/ui/src/forms/button.ds

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_prefers_exact_package_export() {
    let compiler = TestSession::new()
        .data(
            "destack.json",
            r#"
{
    "workspace": {
        "packages": ["packages/*", "packages/@*/*"]
    }
}
"#,
        )
        .data(
            "packages/app/destack.json",
            r#"
{
    "name": "app",
    "dependencies": {
        "@acme/ui": {
            "source": "workspace"
        }
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/destack.json",
            r#"
{
    "name": "@acme/ui",
    "exports": {
        "./button": {
            "kind": "module",
            "path": "button.ds"
        },
        "./*": {
            "kind": "module",
            "path": "src/*.ds"
        }
    }
}
"#,
        )
        .module(
            "packages/app/main.ds",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .module(
            "packages/@acme/ui/button.ds",
            r#"
export type Button = string;
"#,
        )
        .module(
            "packages/@acme/ui/src/button.ds",
            r#"
export type Button = never;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "packages/app/main.ds",
        DirRows::modules().with_summaries(),
        r#"
import { Button } from "@acme/ui/button";
/// @module.edge relation=import specifier=@acme/ui/button module=packages/@acme/ui/button.ds

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_resolves_dependency_enabled_by_condition() {
    let compiler = TestSession::new()
        .data(
            "destack.json",
            r#"
{
    "workspace": {
        "packages": ["packages/*", "packages/@*/*"]
    }
}
"#,
        )
        .data(
            "packages/app/destack.json",
            r#"
{
    "name": "app",
    "conditions": {
        "modes": {
            "preview": {
                "dependencies": {
                    "@acme/ui": {
                        "source": "workspace"
                    }
                }
            }
        }
    },
    "compiler": {
        "modes": ["preview"]
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/destack.json",
            r#"
{
    "name": "@acme/ui",
    "exports": {
        "./button": {
            "kind": "module",
            "path": "button.ds"
        }
    }
}
"#,
        )
        .module(
            "packages/app/main.ds",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .module(
            "packages/@acme/ui/button.ds",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "packages/app/main.ds",
        DirRows::modules().with_summaries(),
        r#"
import { Button } from "@acme/ui/button";
/// @module.edge relation=import specifier=@acme/ui/button module=packages/@acme/ui/button.ds

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_resolves_export_enabled_by_condition() {
    let compiler = TestSession::new()
        .data(
            "destack.json",
            r#"
{
    "workspace": {
        "packages": ["packages/*", "packages/@*/*"]
    }
}
"#,
        )
        .data(
            "packages/app/destack.json",
            r#"
{
    "name": "app",
    "dependencies": {
        "@acme/ui": {
            "source": "workspace"
        }
    },
    "compiler": {
        "modes": ["preview"]
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/destack.json",
            r#"
{
    "name": "@acme/ui",
    "conditions": {
        "modes": {
            "preview": {}
        }
    },
    "exports": {
        "./button": {
            "kind": "module",
            "path": "button.ds",
            "when": "preview"
        }
    }
}
"#,
        )
        .module(
            "packages/app/main.ds",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .module(
            "packages/@acme/ui/button.ds",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "packages/app/main.ds",
        DirRows::modules().with_summaries(),
        r#"
import { Button } from "@acme/ui/button";
/// @module.edge relation=import specifier=@acme/ui/button module=packages/@acme/ui/button.ds

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_resolves_path_dependency() {
    let compiler = TestSession::new()
        .data(
            "destack.json",
            r#"
{
    "workspace": {
        "packages": ["packages/*", "packages/@*/*"]
    }
}
"#,
        )
        .data(
            "packages/app/destack.json",
            r#"
{
    "name": "app",
    "dependencies": {
        "@acme/ui": {
            "source": "path",
            "path": "../@acme/ui"
        }
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/destack.json",
            r#"
{
    "name": "@acme/ui",
    "exports": {
        "./button": {
            "kind": "module",
            "path": "button.ds"
        }
    }
}
"#,
        )
        .module(
            "packages/app/main.ds",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .module(
            "packages/@acme/ui/button.ds",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "packages/app/main.ds",
        DirRows::modules().with_summaries(),
        r#"
import { Button } from "@acme/ui/button";
/// @module.edge relation=import specifier=@acme/ui/button module=packages/@acme/ui/button.ds

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_resolves_path_dependency_outside_workspace_members() {
    let compiler = TestSession::new()
        .data(
            "destack.json",
            r#"
{
    "workspace": {
        "packages": ["packages/app"]
    }
}
"#,
        )
        .data(
            "packages/app/destack.json",
            r#"
{
    "name": "app",
    "dependencies": {
        "@acme/ui": {
            "source": "path",
            "path": "../@acme/ui"
        }
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/destack.json",
            r#"
{
    "name": "@acme/ui",
    "exports": {
        "./button": {
            "kind": "module",
            "path": "button.ds"
        }
    }
}
"#,
        )
        .module(
            "packages/app/main.ds",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .module(
            "packages/@acme/ui/button.ds",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "packages/app/main.ds",
        DirRows::modules().with_summaries(),
        r#"
import { Button } from "@acme/ui/button";
/// @module.edge relation=import specifier=@acme/ui/button module=packages/@acme/ui/button.ds

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_reports_missing_bare_package_dependency() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
import { value } from "pkg";
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error code=EI208 message="package 'pkg' is not declared as a dependency"
/// @diagnostic.label line=2 column=1 source="import { value } from \"pkg\";"
"#,
    );
}

#[test]
fn test_import_reports_missing_scoped_package_dependency() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
import { value } from "@scope/pkg";
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error code=EI208 message="package '@scope/pkg' is not declared as a dependency"
/// @diagnostic.label line=2 column=1 source="import { value } from \"@scope/pkg\";"
"#,
    );
}

#[test]
fn test_import_reports_dependency_disabled_by_condition() {
    let compiler = TestSession::new()
        .data(
            "destack.json",
            r#"
{
    "workspace": {
        "packages": ["packages/*", "packages/@*/*"]
    }
}
"#,
        )
        .data(
            "packages/app/destack.json",
            r#"
{
    "name": "app",
    "conditions": {
        "modes": {
            "preview": {
                "dependencies": {
                    "@acme/ui": {
                        "source": "workspace"
                    }
                }
            }
        }
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/destack.json",
            r#"
{
    "name": "@acme/ui",
    "exports": {
        "./button": {
            "kind": "module",
            "path": "button.ds"
        }
    }
}
"#,
        )
        .module(
            "packages/app/main.ds",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .module(
            "packages/@acme/ui/button.ds",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "packages/app/main.ds",
        r#"
/// @diagnostic.error code=EI208 message="package '@acme/ui' is not declared as a dependency"
/// @diagnostic.label line=2 column=1 source="import { Button } from \"@acme/ui/button\";"
"#,
    );
}

#[test]
fn test_import_reports_export_disabled_by_condition() {
    let compiler = TestSession::new()
        .data(
            "destack.json",
            r#"
{
    "workspace": {
        "packages": ["packages/*", "packages/@*/*"]
    }
}
"#,
        )
        .data(
            "packages/app/destack.json",
            r#"
{
    "name": "app",
    "dependencies": {
        "@acme/ui": {
            "source": "workspace"
        }
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/destack.json",
            r#"
{
    "name": "@acme/ui",
    "conditions": {
        "modes": {
            "preview": {}
        }
    },
    "exports": {
        "./button": {
            "kind": "module",
            "path": "button.ds",
            "when": "preview"
        }
    }
}
"#,
        )
        .module(
            "packages/app/main.ds",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .module(
            "packages/@acme/ui/button.ds",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "packages/app/main.ds",
        r#"
/// @diagnostic.error code=EI209 message="package '@acme/ui' has no active export './button'"
/// @diagnostic.label line=2 column=1 source="import { Button } from \"@acme/ui/button\";"
"#,
    );
}

#[test]
fn test_import_reports_non_module_package_export() {
    let compiler = TestSession::new()
        .data(
            "destack.json",
            r#"
{
    "workspace": {
        "packages": ["packages/*", "packages/@*/*"]
    }
}
"#,
        )
        .data(
            "packages/app/destack.json",
            r#"
{
    "name": "app",
    "dependencies": {
        "@acme/ui": {
            "source": "workspace"
        }
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/destack.json",
            r#"
{
    "name": "@acme/ui",
    "exports": {
        "./button": {
            "kind": "asset",
            "path": "button.css"
        }
    }
}
"#,
        )
        .module(
            "packages/app/main.ds",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "packages/app/main.ds",
        r#"
/// @diagnostic.error code=EI210 message="package '@acme/ui' export './button' is not a module"
/// @diagnostic.label line=2 column=1 source="import { Button } from \"@acme/ui/button\";"
"#,
    );
}

#[test]
fn test_import_reports_cross_package_export_path() {
    let compiler = TestSession::new()
        .data(
            "destack.json",
            r#"
{
    "workspace": {
        "packages": ["packages/*", "packages/@*/*"]
    }
}
"#,
        )
        .data(
            "packages/app/destack.json",
            r#"
{
    "name": "app",
    "dependencies": {
        "@acme/ui": {
            "source": "workspace"
        }
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/destack.json",
            r#"
{
    "name": "@acme/ui",
    "exports": {
        "./button": {
            "kind": "module",
            "path": "../other/button.ds"
        }
    }
}
"#,
        )
        .data(
            "packages/@acme/other/destack.json",
            r#"
{
    "name": "@acme/other"
}
"#,
        )
        .module(
            "packages/app/main.ds",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .module(
            "packages/@acme/other/button.ds",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "packages/app/main.ds",
        r#"
/// @diagnostic.error code=EI207 message="package export path '@acme/ui/button' crosses package boundaries"
/// @diagnostic.label line=2 column=1 source="import { Button } from \"@acme/ui/button\";"
"#,
    );
}
