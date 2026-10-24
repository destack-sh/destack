import type { Environment } from "@destack/environment";
import { checkPackage, fixPackage } from "./check.ts";
import { formatPackage } from "./format.ts";
import { configurePackage } from "./configuration.ts";
import { CheckError } from "../error/index.ts";
import { listAffected } from "../workspace/index.ts";

/** Run the shared package commands in an environment. */
export async function main(
    environment: Environment<"files" | "process" | "platform">,
): Promise<void> {
    // read the command and its source selection
    const { process } = environment;
    let [command = "check", ...files] = process.arguments;

    // retain the conventional read-only formatter flag
    if (command === "format" && files.includes("--check")) {
        command = "format-check";
        files = files.filter((file) => file !== "--check");
    }

    // reject independent tool settings
    if (files.some((file) => file.startsWith("-"))) {
        throw CheckError.of("INVALID_CONFIGURATION", "tool overrides are not supported");
    }

    // write direct-tool settings when requested
    const options = {
        directory: process.workingDirectory,
        environment,
        ...(files.length ? { files } : {}),
    };
    if (command === "configure") {
        await configurePackage(options.directory, environment.files);
        return;
    }

    // print the member directories the changes since a base commit affect
    if (command === "affected") {
        const [base] = files;
        if (base === undefined) {
            throw CheckError.of("INVALID_CONFIGURATION", "affected needs a base commit");
        }
        const directories = await listAffected(options.directory, base, environment);
        await process.stdout.write(
            directories === undefined ? "*\n" : directories.map((path) => `${path}\n`).join(""),
        );

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
        throw CheckError.of("INVALID_CONFIGURATION", `unknown command: ${command}`);
    }

    // report diagnostics or formatter output and preserve failure status
    if ("diagnostics" in result) {
        await process.stdout.write(`${JSON.stringify(result, null, 4)}\n`);
        process.exitCode = result.diagnostics.length > 0 ? 1 : 0;
    } else {
        await process.stdout.write(result.stdout);
        await process.stderr.write(result.stderr);
        process.exitCode = result.code;
    }
}
