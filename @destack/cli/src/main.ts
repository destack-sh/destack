import { BunEnvironment } from "@destack/environment/bun";
import { Release, type ReleaseProcess } from "@destack/update/release";
import { CommandLine } from "./command/line.ts";
import { NativeDevice } from "./command/native.ts";

/** The modules of the processes this program runs besides its command line, by their package. */
const PROCESSES: Readonly<
    Record<
        ReleaseProcess,
        () => Promise<{ readonly main: (words: readonly string[]) => Promise<void> }>
    >
> = {
    machine: () => import("@destack/desktop/machine"),
    build: () => import("@destack/build/main"),
    sandbox: () => import("@destack/sandbox/main"),
    desktop: () => import("@destack/desktop/main"),
};

/** The environment the program runs on: the system's environment. */
const environment = BunEnvironment.system;

/** The process the program starts as, absent for its command line. */
const selected = Release.process(environment.process.arguments);

// run the selected process with the words after its option
if (selected !== undefined) {
    const module = await PROCESSES[selected]();
    await module.main(environment.process.arguments.slice(1));
}
// run the command line on this computer's device and exit with its status
else {
    await using native = new NativeDevice(environment);
    environment.process.exitCode = await CommandLine.run(
        environment.process.arguments,
        () => native.open(),
        environment,
    );
}
