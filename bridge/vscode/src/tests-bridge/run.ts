import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";

import { runTests } from "@vscode/test-electron";

/** Run VSCode bridge smoke tests in a temporary workspace copy. */
async function main() {
    // resolve workspace and compiled test entry paths
    const extensionDevelopmentPath = path.resolve(__dirname, "..", "..");
    const extensionTestsPath = path.resolve(__dirname, "suite", "index");
    const fixtureWorkspacePath = path.resolve(
        extensionDevelopmentPath,
        "src",
        "tests-bridge",
        "fixture",
        "workspace",
    );

    // allocate isolated runtime directories for each run
    const workspacePath = fs.mkdtempSync(path.join(os.tmpdir(), "destack-vscode-bridge-workspace-"));
    const testDataPath = fs.mkdtempSync(path.join(os.tmpdir(), "destack-vscode-bridge-data-"));
    const releaseBridgeRunLock = acquireBridgeRunLock();

    try {
        // copy fixture workspace and write mock server settings
        fs.cpSync(fixtureWorkspacePath, workspacePath, { recursive: true });
        prepareWorkspaceSettings(workspacePath);

        // run tests in vscode integration mode
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
    } finally {
        // clean temporary directories after every run
        fs.rmSync(workspacePath, { recursive: true, force: true });
        fs.rmSync(testDataPath, { recursive: true, force: true });

        // release the run lock after cleanup
        releaseBridgeRunLock();
    }
}

/** Write workspace settings for the bridge smoke fixture. */
function prepareWorkspaceSettings(workspacePath: string): void {
    const settingsDirectory = path.join(workspacePath, ".vscode");
    const settingsPath = path.join(settingsDirectory, "settings.json");
    fs.mkdirSync(settingsDirectory, { recursive: true });

    // always use the local fixture server for bridge smoke tests
    const settings = {
        "destack.server.command": process.execPath,
        "destack.server.args": [path.join(workspacePath, "tools", "fixture-server.js")],
        "workbench.localHistory.enabled": false,
    };
    fs.writeFileSync(settingsPath, `${JSON.stringify(settings, null, 2)}\n`, "utf8");
}

/** Acquire an exclusive lock for one VSCode bridge test process. */
function acquireBridgeRunLock(): () => void {
    // create a stable lock file in the system temp directory
    const lockPath = path.join(os.tmpdir(), "destack-vscode-bridge-run.lock");
    let lockFileDescriptor: number;
    try {
        lockFileDescriptor = fs.openSync(lockPath, "wx");
    } catch (error) {
        // reject concurrent runs with a clear actionable message
        const message = error instanceof Error ? error.message : String(error);
        throw new Error(
            `another VSCode bridge test run is active; wait for it to finish or remove ${lockPath}: ${message}`,
        );
    }

    // record process metadata for easier local cleanup
    fs.writeFileSync(lockFileDescriptor, `${process.pid}\n`, "utf8");

    // release lock on normal completion
    return () => {
        try {
            fs.closeSync(lockFileDescriptor);
        } catch {
            // ignore close failures during cleanup
        }

        fs.rmSync(lockPath, { force: true });
    };
}

// run the test process and fail loudly on errors
main().catch((error) => {
    console.error(error);
    process.exit(1);
});
