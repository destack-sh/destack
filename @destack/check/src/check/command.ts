import process from "node:process";
import { checkPackage, fixPackage } from "./check.ts";
import { formatPackage } from "./format.ts";
import { configurePackage } from "./configuration.ts";
import { CheckError } from "../error/index.ts";

/** Run the shared package commands. */
export async function main(): Promise<void> {
    // read the command and its source selection
    let [command = "check", ...files] = process.argv.slice(2);

    // retain the conventional read-only formatter flag
    if (command === "format" && files.includes("--check")) {
        command = "format-check";
        files = files.filter((file) => file !== "--check");
    }

    // reject independent tool settings
    if (files.some((file) => file.startsWith("-"))) {
        throw new CheckError("configuration", "tool overrides are not supported");
    }

    // write direct-tool settings when requested
    const options = { directory: process.cwd(), files: files.length ? files : undefined };
    if (command === "configure") {
        await configurePackage(options.directory);
        return;
    }

    // select the operation without exposing rule overrides
    let result;
    if (command === "check") {
        result = await checkPackage(options);
    } else if (command === "fix") {
        result = await fixPackage(options);
    } else if (command === "format" || command === "format-check") {
        result = await formatPackage(options, command === "format");
    } else {
        throw new CheckError("configuration", `unknown command: ${command}`);
    }

    // report diagnostics or formatter output and preserve failure status
    if ("diagnostics" in result) {
        process.stdout.write(`${JSON.stringify(result, null, 4)}\n`);
        process.exitCode = result.diagnostics.length > 0 ? 1 : 0;
    } else {
        process.stdout.write(result.stdout);
        process.stderr.write(result.stderr);
        process.exitCode = result.code;
    }
}
