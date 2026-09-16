import { spawnSync } from "node:child_process";
import { mkdirSync, renameSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const generatedDirectory = join(repositoryDirectory, "platform/site/.generated");
const libraryFile = join(generatedDirectory, "library-reference.json");
const lintFile = join(generatedDirectory, "lint-reference.json");

mkdirSync(generatedDirectory, { recursive: true });
generateLintReference();
generateLibraryReference();

/// Generate every registered lint through the public CLI.
function generateLintReference() {
    const output = runDestack(["lint", "--list-rules", "--json"]);

    // require the complete CLI catalog
    if (output.status !== "success" || !Array.isArray(output.data?.items)) {
        throw new Error("destack lint did not produce a rule catalog");
    }

    const reference = {
        schemaVersion: 1,
        rules: output.data.items,
    };

    writeReference(lintFile, reference);
}

/// Generate the Standard Library reference through the public CLI.
function generateLibraryReference() {
    const output = runDestack(["doc", "--cwd", "language/library", "--json"]);

    // require the complete CLI reference
    if (output.status !== "success" || output.data?.reference == undefined) {
        throw new Error("destack doc did not produce a package reference");
    }

    writeReference(libraryFile, output.data.reference);
}

/// Run one Destack CLI command and parse its JSON report.
function runDestack(commandArguments: string[]) {
    const command = spawnSync(
        "cargo",
        [
            "run",
            "--quiet",
            "--package",
            "destack_cli",
            "--bin",
            "destack",
            "--",
            ...commandArguments,
        ],
        {
            cwd: repositoryDirectory,
            encoding: "utf8",
            maxBuffer: 32 * 1024 * 1024,
        },
    );

    // surface process launch failures directly
    if (command.error != undefined) {
        throw command.error;
    }

    // surface command failures with the CLI diagnostics
    if (command.status !== 0) {
        process.stderr.write(command.stderr);
        process.stderr.write(command.stdout);
        throw new Error(`destack ${commandArguments[0]} failed with exit code ${command.status}`);
    }

    return JSON.parse(command.stdout);
}

/// Atomically replace one generated reference.
function writeReference(file: string, reference: unknown) {
    const source = `${JSON.stringify(reference, null, 2)}\n`;
    const temporaryFile = `${file}.${process.pid}.tmp`;
    writeFileSync(temporaryFile, source);
    renameSync(temporaryFile, file);
}
