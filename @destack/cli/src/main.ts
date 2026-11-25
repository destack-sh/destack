import { Environment } from "@destack/environment";
import { BunEnvironment } from "@destack/environment/bun";
import { LineExporter, LOG_LEVEL_VARIABLE } from "@destack/environment/line";
import { Release, type ReleaseProcess } from "@destack/update/release";
import { TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
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

/** The milliseconds in a second. */
const MILLISECONDS_PER_SECOND = 1000;

/** The name the program's command line records its load under, beside its other processes'. */
const COMMAND_LINE = "command";

/** The system's environment, whose variables decide whether the program prints its telemetry. */
const system = BunEnvironment.system;

/** The environment the program runs on, printing its signals as lines when a level is set. */
const environment =
    system.process.variables[LOG_LEVEL_VARIABLE] === undefined ? system : printing(system);

/** The process the program starts as, absent for its command line. */
const selected = Release.process(environment.process.arguments);

// record how long the process took to load its modules, on the clock counting from its start
TelemetryScope.of(import.meta.destack.package, environment.telemetry)
    .metric.histogram("destack.process.load.duration", {
        unit: "s",
        attributes: { "destack.process.name": [COMMAND_LINE, ...Object.keys(PROCESSES)] },
    })
    .record(environment.telemetry.monotonic.now() / MILLISECONDS_PER_SECOND, {
        "destack.process.name": selected ?? COMMAND_LINE,
    });

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

/** Instrument an environment to print its log records and spans as lines on standard error. */
function printing(source: typeof BunEnvironment.system): typeof BunEnvironment.system {
    const exporter = new LineExporter(
        (line) => source.process.stderr.writeSync(line),
        LineExporter.options(source.process.variables),
    );

    return Environment.instrument(
        source,
        exporter.telemetry(source.clock, source.telemetry.context),
    );
}
