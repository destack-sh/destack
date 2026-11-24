import type { Environment } from "@destack/environment";
import { checkPackage, fixPackage } from "./check.ts";
import { formatPackage } from "./format.ts";
import { configurePackage } from "./configuration.ts";
import { CheckError } from "../error/index.ts";
import { listAffected, listChangedModules, Workspace } from "../workspace/index.ts";
import { Schedule, TestStore } from "../project/index.ts";
import { Toolchain } from "../toolchain/index.ts";
import { Readme } from "../markdown/index.ts";

/** Run the shared package commands in an environment. */
export async function main(environment: Environment<"files" | "process" | "host">): Promise<void> {
    // read the command and its source selection
    const { process } = environment;
    let [command = "check", ...files] = process.arguments;

    // retain the conventional read-only formatter flag
    if (command === "format" && files.includes("--check")) {
        command = "format-check";
        files = files.filter((file) => file !== "--check");
    }

    // lint the package's modules changed since a base, none when none changed
    const since = command === "check" && files[0] === "--since" ? files[1] : undefined;
    if (since !== undefined) {
        files = await listChangedModules(process.workingDirectory, since, environment);
        if (files.length === 0) {
            await process.stdout.write(`${JSON.stringify({ diagnostics: [] }, null, 4)}\n`);

            return;
        }
    }

    // reject independent tool settings
    if (files.some((file) => file.startsWith("-"))) {
        throw CheckError.of(
            "CONFIGURATION_INVALID",
            "check argument is invalid: tool overrides are not supported",
        );
    }

    // write direct-tool settings when requested
    const options = {
        directory: process.workingDirectory,
        toolchain: Toolchain.workspace(),
        environment,
        ...(files.length ? { files } : {}),
    };
    if (command === "configure") {
        await configurePackage(options.directory, options.toolchain, environment.files);
        return;
    }

    // print the member directories the changes since a base commit affect
    if (command === "affected") {
        const [base] = files;
        if (base === undefined) {
            throw CheckError.of(
                "CONFIGURATION_INVALID",
                "command 'affected' is invalid: it needs a base commit",
            );
        }
        const directories = await listAffected(options.directory, base, environment);
        await process.stdout.write(
            directories === undefined ? "*\n" : directories.map((path) => `${path}\n`).join(""),
        );

        return;
    }

    // print the CI runners for packages: their checks and the test files that have not passed
    if (command === "schedule") {
        const workspace = await Workspace.read(options.directory, environment.files);
        if (workspace === undefined) {
            throw CheckError.of(
                "CONFIGURATION_INVALID",
                `command 'schedule' is invalid: '${options.directory}' is in no workspace`,
            );
        }
        const store = await TestStore.open(environment);
        const schedule = await Schedule.of(workspace.root, files, store, environment);
        await process.stdout.write(`${JSON.stringify(schedule.runners)}\n`);

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
    } else if (command === "readme") {
        result = await Readme.check(options.directory, options.toolchain, environment);
    } else {
        throw CheckError.of(
            "CONFIGURATION_INVALID",
            `command '${command}' is invalid: it must be check, fix, format, format-check, configure, readme, affected or schedule`,
        );
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
