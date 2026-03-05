import { spawnSync } from "node:child_process";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { runTests } from "@vscode/test-electron";

/** Run extension host integration tests in a temporary workspace copy. */
async function main() {
    // resolve workspace and test entry paths
    const extensionDevelopmentPath = path.resolve(__dirname, "..", "..");
    const extensionTestsPath = path.resolve(__dirname, "suite", "index");
    const fixtureWorkspacePath = path.resolve(
        extensionDevelopmentPath,
        "typescript",
        "tests-host",
        "fixture",
        "workspace",
    );

    // allocate isolated runtime directories
    const workspacePath = fs.mkdtempSync(path.join(os.tmpdir(), "destack-vscode-workspace-"));
    const testDataPath = fs.mkdtempSync(path.join(os.tmpdir(), "destack-vscode-host-test-"));
    const releaseHostRunLock = acquireHostRunLock();

    try {
        // keep real-server runs hermetic by building the local server binary when requested
        await ensureRealServerCommandReady(extensionDevelopmentPath);

        // copy fixture workspace and write dynamic settings
        fs.cpSync(fixtureWorkspacePath, workspacePath, { recursive: true });
        prepareWorkspaceSettings(workspacePath);

        // run tests in VSCode extension host
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
        releaseHostRunLock();
    }
}

/** Write workspace settings for mock server or real server mode. */
function prepareWorkspaceSettings(workspacePath: string): void {
    // read server command mode from the environment
    const realServerCommand = (process.env.DESTACK_VSCODE_REAL_SERVER_COMMAND ?? "").trim();
    const useRealServer = realServerCommand.length > 0;
    const settingsDirectory = path.join(workspacePath, ".vscode");
    const settingsPath = path.join(settingsDirectory, "settings.json");
    fs.mkdirSync(settingsDirectory, { recursive: true });

    // build launch settings for the selected mode
    const serverCommand = useRealServer ? realServerCommand : process.execPath;
    const serverArgs = useRealServer
        ? parseServerArgs(process.env.DESTACK_VSCODE_REAL_SERVER_ARGS)
        : [path.join(workspacePath, "tools", "mock-lsp.js")];
    const settings = {
        "destack.server.command": serverCommand,
        "destack.server.args": serverArgs,
        "workbench.localHistory.enabled": false,
    };
    fs.writeFileSync(settingsPath, `${JSON.stringify(settings, null, 2)}\n`, "utf8");

    // expose real server mode to test suites
    if (useRealServer) {
        process.env.DESTACK_VSCODE_HOST_REAL_SERVER = "1";
    } else {
        delete process.env.DESTACK_VSCODE_HOST_REAL_SERVER;
    }
}

/** Acquire an exclusive lock for one host test run process. */
function acquireHostRunLock(): () => void {
    // create a stable lock file in the system temp directory
    const lockPath = path.join(os.tmpdir(), "destack-vscode-host-run.lock");
    let lockFileDescriptor: number;
    try {
        lockFileDescriptor = fs.openSync(lockPath, "wx");
    } catch (error) {
        // reject concurrent runs with a clear actionable message
        const message = error instanceof Error ? error.message : String(error);
        throw new Error(
            `another VSCode host test run is active; wait for it to finish or remove ${lockPath}: ${message}`,
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

/** Ensure the real server command is built when it targets the local workspace binary. */
async function ensureRealServerCommandReady(extensionDevelopmentPath: string): Promise<void> {
    // skip when tests run in mock mode
    const realServerCommand = (process.env.DESTACK_VSCODE_REAL_SERVER_COMMAND ?? "").trim();
    if (realServerCommand.length == 0) {
        return;
    }

    // allow disabling automatic build from the environment
    if (process.env.DESTACK_VSCODE_REAL_SERVER_AUTO_BUILD == "0") {
        return;
    }

    // only auto-build for local workspace target binaries
    const repositoryRoot = path.resolve(extensionDevelopmentPath, "..", "..");
    const normalizedCommandPath = path.resolve(realServerCommand);
    const isWorkspaceCommand = normalizedCommandPath.startsWith(`${repositoryRoot}${path.sep}`);
    const isTargetBinary = normalizedCommandPath.includes(`${path.sep}target${path.sep}`);
    const commandName = path.basename(normalizedCommandPath);
    if (!isWorkspaceCommand || !isTargetBinary || commandName != "destack") {
        return;
    }

    // use release profile when the selected command path points to target/release
    const useReleaseProfile = normalizedCommandPath.includes(
        `${path.sep}target${path.sep}release${path.sep}`,
    );
    const buildArguments = ["build", "-p", "destack_cli"];
    if (useReleaseProfile) {
        buildArguments.push("--release");
    }

    // build the server binary before launching VSCode host tests
    const buildResult = spawnSync("cargo", buildArguments, {
        cwd: repositoryRoot,
        stdio: "inherit",
    });
    if (buildResult.status !== 0) {
        throw new Error(`failed to build real server command: ${normalizedCommandPath}`);
    }
}

/** Parse optional JSON or whitespace separated real server arguments. */
function parseServerArgs(rawArgs: string | undefined): string[] {
    // normalize the raw value first
    const value = rawArgs?.trim();
    if (!value) {
        return [];
    }

    // parse JSON array form when available
    let parseError: unknown;
    try {
        const parsed = JSON.parse(value);
        if (Array.isArray(parsed) && parsed.every((item) => typeof item == "string")) {
            return parsed;
        }
    } catch (error) {
        parseError = error;
    }

    // report JSON parse errors before fallback parsing
    if (parseError) {
        const message = parseError instanceof Error ? parseError.message : String(parseError);
        console.warn(
            `DESTACK_VSCODE_REAL_SERVER_ARGS is not valid JSON string[]; using whitespace split fallback: ${message}`,
        );
    }

    // fallback to whitespace separated args
    return value.split(/\s+/).filter((item) => item.length > 0);
}

// run the host test process and fail loudly on errors
main().catch((error) => {
    console.error(error);
    process.exit(1);
});
