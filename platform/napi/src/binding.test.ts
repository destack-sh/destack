import { describe, expect, test } from "bun:test";
import { mkdtempSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
    checkSync,
    defaultRunOptions,
    defaultCheckOptions,
    defaultCompilerOptions,
    defaultFormatOptions,
    defaultLinterOptions,
    defaultLinterRequestOptions,
    defaultParseOptions,
    defaultResolveOptions,
    defaultTransformOptions,
    FileType,
    formatSync,
    lintSync,
    ModuleFormat,
    ModuleType,
    parseSync,
    resolveSync,
    runSync,
    RunInputKind,
    transformSync,
} from "./index";

function createTemporaryWorkspace(): string {
    // create a temporary workspace root
    return mkdtempSync(join(tmpdir(), "destack-napi-test-"));
}

function removeTemporaryWorkspace(path: string): void {
    // clean up the temporary workspace root
    rmSync(path, { recursive: true, force: true });
}

function createSourceFixture(root: string): {
    entryPath: string;
    dependencyPath: string;
    code: string;
} {
    // build fixture file paths
    const entryPath = join(root, "main.ts");
    const dependencyPath = join(root, "dependency.ts");
    const code = "export const value: number = 1\n";

    // write fixture files
    writeFileSync(entryPath, code);
    writeFileSync(dependencyPath, "export const dependency = 1\n");

    return { entryPath, dependencyPath, code };
}

describe("napi", () => {
    test("test_default_options_are_usable", () => {
        // returns usable defaults for compiler and tooling options
        const compiler = defaultCompilerOptions();
        expect(typeof compiler.workers).toBe("number");
        expect(typeof compiler.followImports).toBe("boolean");
        expect(typeof compiler.verifyMir).toBe("boolean");

        // returns usable defaults for resolver options
        const resolveOptions = defaultResolveOptions();
        expect(Array.isArray(resolveOptions.extensions)).toBe(true);

        // returns usable defaults for parse options
        const parseOptions = defaultParseOptions();
        expect(typeof parseOptions.cwd).toBe("string");
        expect(Array.isArray(parseOptions.roots)).toBe(true);

        // returns usable defaults for transform options
        const transformOptions = defaultTransformOptions();
        expect(typeof transformOptions.cwd).toBe("string");
        expect(Array.isArray(transformOptions.roots)).toBe(true);

        // returns usable defaults for check options
        const checkOptions = defaultCheckOptions();
        expect(typeof checkOptions.cwd).toBe("string");
        expect(Array.isArray(checkOptions.roots)).toBe(true);

        // returns usable defaults for linter options
        const linterOptions = defaultLinterOptions();
        expect(typeof linterOptions.enabled).toBe("boolean");

        // returns usable defaults for linter request options
        const linterRequestOptions = defaultLinterRequestOptions();
        expect(typeof linterRequestOptions.cwd).toBe("string");
        expect(Array.isArray(linterRequestOptions.roots)).toBe(true);

        // returns usable defaults for format options
        const formatOptions = defaultFormatOptions();
        expect(typeof formatOptions.indentWidth).toBe("number");
    });

    test("test_shared_file_and_module_nouns_are_exposed", () => {
        // exposes shared file and module noun enums
        expect(typeof FileType.TypeScript).toBe("number");
        expect(typeof ModuleType.Code).toBe("number");
        expect(typeof ModuleFormat.EsNext).toBe("number");
    });

    test("test_resolve_sync_accepts_file_path_for_from", () => {
        // resolves a relative dependency from a file path
        const root = createTemporaryWorkspace();
        try {
            const { entryPath, dependencyPath } = createSourceFixture(root);

            // resolve and verify the full target path
            const result = resolveSync("./dependency.ts", entryPath, {
                ...defaultResolveOptions(),
                cwd: root,
            });

            expect(realpathSync(result.path)).toBe(realpathSync(dependencyPath));
        } finally {
            removeTemporaryWorkspace(root);
        }
    });

    test("test_parse_sync_returns_program_payload", () => {
        // parses source and returns program payload metadata
        const root = createTemporaryWorkspace();
        try {
            const { entryPath, code } = createSourceFixture(root);

            // parse and verify program shape
            const result = parseSync(entryPath, code, {
                ...defaultParseOptions(),
                cwd: root,
                roots: [root],
            });

            expect(result.program.path).toBe(entryPath);
            expect(result.program.rootCount).toBeGreaterThan(0);
            expect(() => JSON.parse(result.program.astJson)).not.toThrow();
        } finally {
            removeTemporaryWorkspace(root);
        }
    });

    test("test_transform_sync_emits_code", () => {
        // transforms source and returns emitted code
        const root = createTemporaryWorkspace();
        try {
            const { entryPath, code } = createSourceFixture(root);

            // transform and verify emitted output
            const result = transformSync(entryPath, code, {
                ...defaultTransformOptions(),
                cwd: root,
                roots: [root],
            });

            expect(result.code.length).toBeGreaterThan(0);
            expect(result.code).toContain("value");
        } finally {
            removeTemporaryWorkspace(root);
        }
    });

    test("test_check_sync_reports_errors_for_invalid_source", () => {
        // reports diagnostics for invalid input
        const root = createTemporaryWorkspace();
        try {
            const sourcePath = join(root, "invalid.ts");
            const code = "const = 1\n";

            // check and verify error diagnostics are reported
            const result = checkSync(sourcePath, code, {
                ...defaultCheckOptions(),
                cwd: root,
                roots: [root],
            });

            expect(result.hasErrors).toBe(true);
            expect(result.diagnosticCount).toBeGreaterThan(0);
        } finally {
            removeTemporaryWorkspace(root);
        }
    });

    test("test_lint_sync_reports_diagnostics_for_invalid_source", () => {
        // reports lint diagnostics for invalid input
        const root = createTemporaryWorkspace();
        try {
            const sourcePath = join(root, "invalid.ts");
            const code = "const = 1\n";

            // lint and verify diagnostics are reported
            const result = lintSync(sourcePath, code, {
                ...defaultLinterRequestOptions(),
                cwd: root,
                roots: [root],
            });

            expect(result.hasErrors).toBe(true);
            expect(result.diagnosticCount).toBeGreaterThan(0);
        } finally {
            removeTemporaryWorkspace(root);
        }
    });

    test("test_format_sync_is_deterministic", () => {
        // formatting the same source twice yields the same output
        const sourcePath = "/tmp/format.ts";
        const source = "export const value=1\n";

        // format once
        const formatted = formatSync(sourcePath, source, defaultFormatOptions());
        expect(formatted.code.length).toBeGreaterThan(0);

        // format the formatted output again
        const formattedAgain = formatSync(sourcePath, formatted.code, defaultFormatOptions());
        expect(formattedAgain.code).toBe(formatted.code);
    });

    test("test_run_sync_validates_input_contract", () => {
        // rejects file input when path is missing
        const options = defaultRunOptions();
        const input = { kind: RunInputKind.File };

        expect(() => runSync(input, options)).toThrow("file input requires path");
    });

    test("test_run_sync_accepts_object_override_payloads", () => {
        // accepts object override payloads and still validates required fields
        const options = {
            ...defaultRunOptions(),
            targetOverrides: {},
            runtimeOverrides: {},
        };
        const input = { kind: RunInputKind.File };

        expect(() => runSync(input, options)).toThrow("file input requires path");
    });

    test("test_run_sync_rejects_unserializable_override_payloads", () => {
        // rejects cyclic override payloads
        const circular: { self?: unknown } = {};
        circular.self = circular;

        const options = {
            ...defaultRunOptions(),
            targetOverrides: circular,
        };
        const input = { kind: RunInputKind.File };

        expect(() => runSync(input, options)).toThrow("cyclic");
    });
});
