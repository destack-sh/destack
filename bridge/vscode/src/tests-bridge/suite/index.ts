import * as fs from "node:fs";
import * as path from "node:path";

import Mocha from "mocha";

/** Ordered suite files loaded by the VSCode test runner. */
const ORDERED_SUITE_FILES = ["bridge.test.js"];

/** Run the bridge smoke suite entrypoint for VSCode integration tests. */
export async function run(): Promise<void> {
    // configure mocha for vscode integration execution
    const mocha = new Mocha({
        ui: "tdd",
        color: true,
        timeout: 60_000,
    });

    // add all compiled suite files in deterministic order
    const suiteFiles = orderedSuiteFiles(__dirname);
    for (const suiteFile of suiteFiles) {
        mocha.addFile(suiteFile);
    }

    // execute the run and surface failures as errors
    await new Promise<void>((resolve, reject) => {
        mocha.run((failures) => {
            // reject when at least one test fails
            if (failures > 0) {
                reject(new Error(`${failures} VSCode bridge tests failed`));
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

    return suitePaths;
}
