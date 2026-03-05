import * as fs from "node:fs";
import * as path from "node:path";
import Mocha from "mocha";

/** Ordered host-suite files loaded by the extension test runner. */
const ORDERED_SUITE_FILES = [
    "smoke.test.js",
    "diag.test.js",
    "command.test.js",
    "flow.test.js",
    "rename.test.js",
    "workspace.test.js",
];

/** Run the host suite entrypoint for VSCode extension tests. */
export async function run(): Promise<void> {
    // configure mocha for extension host execution
    const mocha = new Mocha({
        ui: "tdd",
        color: true,
        timeout: 60_000,
    });

    // add all compiled test files in a deterministic order
    const suiteFiles = orderedSuiteFiles(__dirname);
    for (const suiteFile of suiteFiles) {
        mocha.addFile(suiteFile);
    }

    // execute the mocha run and surface failures as errors
    await new Promise<void>((resolve, reject) => {
        mocha.run((failures) => {
            // reject when at least one test fails
            if (failures > 0) {
                reject(new Error(`${failures} VSCode host tests failed`));
                return;
            }

            resolve();
        });
    });
}

/** Resolve ordered suite files and validate each expected file exists. */
function orderedSuiteFiles(directory: string): string[] {
    // map known suite file names to absolute paths
    const suitePaths = ORDERED_SUITE_FILES.map((fileName) => path.join(directory, fileName));

    // fail loudly when a listed suite is missing
    for (const suitePath of suitePaths) {
        if (!fs.existsSync(suitePath)) {
            throw new Error(`missing suite file: ${suitePath}`);
        }
    }

    // return the full suite list when no selection filter exists
    const selectedSuiteNames = selectedSuiteFileNames();
    if (selectedSuiteNames.length == 0) {
        return suitePaths;
    }

    // reject unknown suite names to keep ci and local runs deterministic
    for (const selectedSuiteName of selectedSuiteNames) {
        if (!ORDERED_SUITE_FILES.includes(selectedSuiteName)) {
            throw new Error(
                `unknown suite '${selectedSuiteName}'; expected one of ${ORDERED_SUITE_FILES.join(", ")}`,
            );
        }
    }

    // keep only selected suites in deterministic order
    return suitePaths.filter((suitePath) => {
        const fileName = path.basename(suitePath);
        return selectedSuiteNames.includes(fileName);
    });
}

/** Parse optional suite filters from DESTACK_VSCODE_HOST_SUITES. */
function selectedSuiteFileNames(): string[] {
    // parse optional comma-separated file list
    const rawSelection = process.env.DESTACK_VSCODE_HOST_SUITES?.trim();
    if (!rawSelection) {
        return [];
    }

    // normalize each suite name to the compiled file naming scheme
    return rawSelection
        .split(",")
        .map((value) => value.trim())
        .filter((value) => value.length > 0)
        .map((value) => (value.endsWith(".test.js") ? value : `${value}.test.js`));
}
