import { BunEnvironment } from "@destack/environment/bun";
import { CommandLine } from "./command/line.ts";
import { NativeDevice } from "./command/native.ts";

/** The environment the command line runs on: the system's environment. */
const environment = BunEnvironment.system;

/** This computer's device, closed once the command line ran. */
await using native = new NativeDevice(environment);

// run the command line and exit with its status
environment.process.exitCode = await CommandLine.run(
    environment.process.arguments,
    () => native.open(),
    environment,
);
