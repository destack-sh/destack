import * as path from "node:path";
import * as fs from "node:fs";
import { runTests } from "@vscode/test-electron";

async function main() {
    const extensionDevelopmentPath = path.resolve(__dirname, "..", "..");
    const extensionTestsPath = path.resolve(__dirname, "suite", "index");
    const workspacePath = path.resolve(
        extensionDevelopmentPath,
        "typescript",
        "tests-host",
        "fixture",
        "workspace",
    );
    const testDataPath = path.resolve(
        "/tmp",
        "destack-vscode-host-test",
    );
    fs.mkdirSync(testDataPath, { recursive: true });

    await runTests({
        extensionDevelopmentPath,
        extensionTestsPath,
        launchArgs: [
            workspacePath,
            "--disable-extensions",
            "--skip-welcome",
            "--skip-release-notes",
            "--disable-workspace-trust",
            "--disable-gpu",
            `--user-data-dir=${testDataPath}`,
            `--extensions-dir=${path.join(testDataPath, "extensions")}`,
        ],
    });
}

main().catch((error) => {
    console.error(error);
    process.exit(1);
});
