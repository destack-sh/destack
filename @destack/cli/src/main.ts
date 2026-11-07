import { BunEnvironment } from "@destack/environment/bun";
import { Device } from "@destack/machine/device";
import { CommandLine } from "./command/run.ts";

/** The environment the command line runs on: the system's environment. */
const environment = BunEnvironment.system;

// run the command line and exit with its status
environment.process.exitCode = await CommandLine.run(
    environment.process.arguments,
    () => Device.current(environment),
    environment,
);
