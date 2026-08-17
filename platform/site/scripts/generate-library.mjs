import { spawnSync } from "node:child_process";
import { mkdirSync, renameSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const generatedDirectory = join(repositoryDirectory, "platform/site/.generated");
const referenceFile = join(generatedDirectory, "library-reference.json");

// generate the checked package reference through the public CLI
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
        "doc",
        "--cwd",
        "language/library",
        "--json",
    ],
    {
        cwd: repositoryDirectory,
        encoding: "utf8",
        maxBuffer: 32 * 1024 * 1024,
    },
);
if (command.error != undefined) {
    throw command.error;
}
if (command.status !== 0) {
    process.stderr.write(command.stderr);
    throw new Error(`destack doc failed with exit code ${command.status}`);
}

// retain only the versioned package artifact
const output = JSON.parse(command.stdout);
if (output.status !== "success" || output.data?.reference == undefined) {
    throw new Error("destack doc did not produce a package reference");
}
const source = `${JSON.stringify(output.data.reference, null, 2)}\n`;

// atomically replace the local build input
mkdirSync(generatedDirectory, { recursive: true });
const temporaryFile = `${referenceFile}.${process.pid}.tmp`;
writeFileSync(temporaryFile, source);
renameSync(temporaryFile, referenceFile);
