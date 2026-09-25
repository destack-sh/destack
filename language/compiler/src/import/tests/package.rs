use crate::tests::{DirRows, TestSession};

#[test]
fn test_import_resolves_package_export() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "workspaces": ["packages/*", "packages/@*/*"]
}
"#,
        )
        .data(
            "packages/app/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "app",
    "dependencies": {
        "@acme/ui": "workspace:*"
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "@acme/ui",
    "exports": {
        "./button": "button.tspp"
    }
}
"#,
        )
        .module(
            "packages/app/main.tspp",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .module(
            "packages/@acme/ui/button.tspp",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "packages/app/main.tspp",
        DirRows::modules().with_summaries(),
        r#"
import { Button } from "@acme/ui/button";
/// @module.edge relation=import specifier=@acme/ui/button module=packages/@acme/ui/button.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_resolves_package_root_export() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "workspaces": ["packages/*", "packages/@*/*"]
}
"#,
        )
        .data(
            "packages/app/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "app",
    "dependencies": {
        "@acme/ui": "workspace:*"
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "@acme/ui",
    "exports": {
        ".": "index.tspp"
    }
}
"#,
        )
        .module(
            "packages/app/main.tspp",
            r#"
import { Button } from "@acme/ui";
"#,
        )
        .module(
            "packages/@acme/ui/index.tspp",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "packages/app/main.tspp",
        DirRows::modules().with_summaries(),
        r#"
import { Button } from "@acme/ui";
/// @module.edge relation=import specifier=@acme/ui module=packages/@acme/ui/index.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_resolves_package_pattern_export() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "workspaces": ["packages/*", "packages/@*/*"]
}
"#,
        )
        .data(
            "packages/app/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "app",
    "dependencies": {
        "@acme/ui": "workspace:*"
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "@acme/ui",
    "exports": {
        "./*": "src/*.tspp"
    }
}
"#,
        )
        .module(
            "packages/app/main.tspp",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .module(
            "packages/@acme/ui/src/button.tspp",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "packages/app/main.tspp",
        DirRows::modules().with_summaries(),
        r#"
import { Button } from "@acme/ui/button";
/// @module.edge relation=import specifier=@acme/ui/button module=packages/@acme/ui/src/button.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_resolves_nested_package_pattern_export() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "workspaces": ["packages/*", "packages/@*/*"]
}
"#,
        )
        .data(
            "packages/app/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "app",
    "dependencies": {
        "@acme/ui": "workspace:*"
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "@acme/ui",
    "exports": {
        "./*": "src/*.tspp"
    }
}
"#,
        )
        .module(
            "packages/app/main.tspp",
            r#"
import { Button } from "@acme/ui/forms/button";
"#,
        )
        .module(
            "packages/@acme/ui/src/forms/button.tspp",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "packages/app/main.tspp",
        DirRows::modules().with_summaries(),
        r#"
import { Button } from "@acme/ui/forms/button";
/// @module.edge relation=import specifier=@acme/ui/forms/button module=packages/@acme/ui/src/forms/button.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_prefers_exact_package_export() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "workspaces": ["packages/*", "packages/@*/*"]
}
"#,
        )
        .data(
            "packages/app/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "app",
    "dependencies": {
        "@acme/ui": "workspace:*"
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "@acme/ui",
    "exports": {
        "./button": "button.tspp",
        "./*": "src/*.tspp"
    }
}
"#,
        )
        .module(
            "packages/app/main.tspp",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .module(
            "packages/@acme/ui/button.tspp",
            r#"
export type Button = string;
"#,
        )
        .module(
            "packages/@acme/ui/src/button.tspp",
            r#"
export type Button = never;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "packages/app/main.tspp",
        DirRows::modules().with_summaries(),
        r#"
import { Button } from "@acme/ui/button";
/// @module.edge relation=import specifier=@acme/ui/button module=packages/@acme/ui/button.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_resolves_dependency_enabled_by_condition() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "workspaces": ["packages/*", "packages/@*/*"]
}
"#,
        )
        .data(
            "packages/app/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "app",
    "conditions": {
        "modes": {
            "preview": {
                "dependencies": {
                    "@acme/ui": "workspace:*"
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
            "packages/@acme/ui/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "@acme/ui",
    "exports": {
        "./button": "button.tspp"
    }
}
"#,
        )
        .module(
            "packages/app/main.tspp",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .module(
            "packages/@acme/ui/button.tspp",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "packages/app/main.tspp",
        DirRows::modules().with_summaries(),
        r#"
import { Button } from "@acme/ui/button";
/// @module.edge relation=import specifier=@acme/ui/button module=packages/@acme/ui/button.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_resolves_dependency_enabled_by_any_condition() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "workspaces": ["packages/*", "packages/@*/*"]
}
"#,
        )
        .data(
            "packages/app/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "app",
    "conditionalDependencies": [
        {
            "when": {
                "any": ["role:client", "mode:preview"]
            },
            "dependencies": {
                "@acme/ui": "workspace:*"
            }
        }
    ],
    "compiler": {
        "modes": ["preview"]
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "@acme/ui",
    "exports": {
        "./button": "button.tspp"
    }
}
"#,
        )
        .module(
            "packages/app/main.tspp",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .module(
            "packages/@acme/ui/button.tspp",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "packages/app/main.tspp",
        DirRows::modules().with_summaries(),
        r#"
import { Button } from "@acme/ui/button";
/// @module.edge relation=import specifier=@acme/ui/button module=packages/@acme/ui/button.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_resolves_export_enabled_by_condition() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "workspaces": ["packages/*", "packages/@*/*"]
}
"#,
        )
        .data(
            "packages/app/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "app",
    "dependencies": {
        "@acme/ui": "workspace:*"
    },
    "compiler": {
        "modes": ["preview"]
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "@acme/ui",
    "conditions": {
        "modes": {
            "preview": {}
        }
    },
    "exports": {
        "./button": { "preview": "button.tspp" }
    }
}
"#,
        )
        .module(
            "packages/app/main.tspp",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .module(
            "packages/@acme/ui/button.tspp",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "packages/app/main.tspp",
        DirRows::modules().with_summaries(),
        r#"
import { Button } from "@acme/ui/button";
/// @module.edge relation=import specifier=@acme/ui/button module=packages/@acme/ui/button.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_reports_export_disabled_by_not_condition() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "workspaces": ["packages/*", "packages/@*/*"]
}
"#,
        )
        .data(
            "packages/app/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "app",
    "dependencies": {
        "@acme/ui": "workspace:*"
    },
    "compiler": {
        "modes": ["preview"]
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "@acme/ui",
    "conditions": {
        "aliases": {
            "stable": {
                "not": "mode:preview"
            }
        }
    },
    "exports": {
        "./button": { "stable": "button.tspp" }
    }
}
"#,
        )
        .module(
            "packages/app/main.tspp",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .module(
            "packages/@acme/ui/button.tspp",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "packages/app/main.tspp",
        r#"
/// @diagnostic.error id=missing-package-export message="package '@acme/ui' has no active export './button'"
/// @diagnostic.label line=2 column=1 span="import { Button } from \"@acme/ui/button\"" line_source="import { Button } from \"@acme/ui/button\";"
"#,
    );
}

#[test]
fn test_import_resolves_path_dependency() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "workspaces": ["packages/*", "packages/@*/*"]
}
"#,
        )
        .data(
            "packages/app/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "app",
    "dependencies": {
        "@acme/ui": "file:../@acme/ui"
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "@acme/ui",
    "exports": {
        "./button": "button.tspp"
    }
}
"#,
        )
        .module(
            "packages/app/main.tspp",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .module(
            "packages/@acme/ui/button.tspp",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "packages/app/main.tspp",
        DirRows::modules().with_summaries(),
        r#"
import { Button } from "@acme/ui/button";
/// @module.edge relation=import specifier=@acme/ui/button module=packages/@acme/ui/button.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_resolves_path_dependency_outside_workspace_members() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "workspaces": ["packages/app"]
}
"#,
        )
        .data(
            "packages/app/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "app",
    "dependencies": {
        "@acme/ui": "file:../@acme/ui"
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "@acme/ui",
    "exports": {
        "./button": "button.tspp"
    }
}
"#,
        )
        .module(
            "packages/app/main.tspp",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .module(
            "packages/@acme/ui/button.tspp",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "packages/app/main.tspp",
        DirRows::modules().with_summaries(),
        r#"
import { Button } from "@acme/ui/button";
/// @module.edge relation=import specifier=@acme/ui/button module=packages/@acme/ui/button.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_reports_missing_bare_package_dependency() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { value } from "pkg";
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=missing-package-dependency message="package 'pkg' is not declared as a dependency"
/// @diagnostic.label line=2 column=1 span="import { value } from \"pkg\"" line_source="import { value } from \"pkg\";"
"#,
    );
}

#[test]
fn test_import_reports_missing_scoped_package_dependency() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { value } from "@scope/pkg";
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=missing-package-dependency message="package '@scope/pkg' is not declared as a dependency"
/// @diagnostic.label line=2 column=1 span="import { value } from \"@scope/pkg\"" line_source="import { value } from \"@scope/pkg\";"
"#,
    );
}

#[test]
fn test_import_reports_dependency_disabled_by_condition() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "workspaces": ["packages/*", "packages/@*/*"]
}
"#,
        )
        .data(
            "packages/app/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "app",
    "conditions": {
        "modes": {
            "preview": {
                "dependencies": {
                    "@acme/ui": "workspace:*"
                }
            }
        }
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "@acme/ui",
    "exports": {
        "./button": "button.tspp"
    }
}
"#,
        )
        .module(
            "packages/app/main.tspp",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .module(
            "packages/@acme/ui/button.tspp",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "packages/app/main.tspp",
        r#"
/// @diagnostic.error id=missing-package-dependency message="package '@acme/ui' is not declared as a dependency"
/// @diagnostic.label line=2 column=1 span="import { Button } from \"@acme/ui/button\"" line_source="import { Button } from \"@acme/ui/button\";"
"#,
    );
}

#[test]
fn test_import_reports_export_disabled_by_condition() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "workspaces": ["packages/*", "packages/@*/*"]
}
"#,
        )
        .data(
            "packages/app/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "app",
    "dependencies": {
        "@acme/ui": "workspace:*"
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "@acme/ui",
    "conditions": {
        "modes": {
            "preview": {}
        }
    },
    "exports": {
        "./button": { "preview": "button.tspp" }
    }
}
"#,
        )
        .module(
            "packages/app/main.tspp",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .module(
            "packages/@acme/ui/button.tspp",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "packages/app/main.tspp",
        r#"
/// @diagnostic.error id=missing-package-export message="package '@acme/ui' has no active export './button'"
/// @diagnostic.label line=2 column=1 span="import { Button } from \"@acme/ui/button\"" line_source="import { Button } from \"@acme/ui/button\";"
"#,
    );
}

#[test]
fn test_import_reports_non_module_package_export() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "workspaces": ["packages/*", "packages/@*/*"]
}
"#,
        )
        .data(
            "packages/app/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "app",
    "dependencies": {
        "@acme/ui": "workspace:*"
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "@acme/ui",
    "exports": {
        "./button": "button.css"
    }
}
"#,
        )
        .module(
            "packages/app/main.tspp",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "packages/app/main.tspp",
        r#"
/// @diagnostic.error id=non-module-package-export message="package '@acme/ui' export './button' is not a module"
/// @diagnostic.label line=2 column=1 span="import { Button } from \"@acme/ui/button\"" line_source="import { Button } from \"@acme/ui/button\";"
"#,
    );
}

#[test]
fn test_import_reports_cross_package_export_path() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "workspaces": ["packages/*", "packages/@*/*"]
}
"#,
        )
        .data(
            "packages/app/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "app",
    "dependencies": {
        "@acme/ui": "workspace:*"
    }
}
"#,
        )
        .data(
            "packages/@acme/ui/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "@acme/ui",
    "exports": {
        "./button": "../other/button.tspp"
    }
}
"#,
        )
        .data(
            "packages/@acme/other/package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "@acme/other"
}
"#,
        )
        .module(
            "packages/app/main.tspp",
            r#"
import { Button } from "@acme/ui/button";
"#,
        )
        .module(
            "packages/@acme/other/button.tspp",
            r#"
export type Button = string;
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "packages/app/main.tspp",
        r#"
/// @diagnostic.error id=cross-package-export message="package export path '@acme/ui/button' crosses package boundaries"
/// @diagnostic.label line=2 column=1 span="import { Button } from \"@acme/ui/button\"" line_source="import { Button } from \"@acme/ui/button\";"
"#,
    );
}
