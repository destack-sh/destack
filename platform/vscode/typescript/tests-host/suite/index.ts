import * as path from "node:path";
import Mocha from "mocha";

export async function run(): Promise<void> {
    const mocha = new Mocha({
        ui: "tdd",
        color: true,
        timeout: 60_000,
    });

    const testPath = path.resolve(__dirname, "smoke.test");
    mocha.addFile(testPath);

    await new Promise<void>((resolve, reject) => {
        mocha.run((failures) => {
            if (failures > 0) {
                reject(new Error(`${failures} VSCode host smoke tests failed`));
                return;
            }

            resolve();
        });
    });
}
