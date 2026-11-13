import { SpanKind, SpanStatusCode } from "@opentelemetry/api";
import { ENVIRONMENT_SCOPE, DURATION_BUCKETS, SpanRunner } from "../telemetry/runner.ts";
import type { Telemetry } from "../telemetry/telemetry.ts";
import type { Process, Subprocess } from "./process.ts";

/** The milliseconds in a second, as OpenTelemetry records durations in seconds. */
const MILLISECONDS_PER_SECOND = 1000;

/** A process running each child in a span from spawn to exit, its duration by executable. */
export class InstrumentedProcess implements Process {
    /** The process the members go to. */
    readonly source: Process;
    /** The runner of whole commands' spans. */
    readonly #span: SpanRunner;
    /** The tracer of spawned and forked children. */
    readonly #children: ReturnType<typeof childrenOf>;

    /** Instrument a process's children on a telemetry, passing every other member through. */
    constructor(source: Process, telemetry: Telemetry) {
        this.source = source;
        this.#span = SpanRunner.of(
            telemetry.tracers.getTracer(ENVIRONMENT_SCOPE),
            telemetry.meters.getMeter(ENVIRONMENT_SCOPE),
            telemetry,
        );
        this.#children = childrenOf(telemetry);
    }

    /** Read the uninstrumented process below one, itself when uninstrumented. */
    static sourceOf(process: Process): Process {
        return process instanceof InstrumentedProcess ? process.source : process;
    }

    /** Run a command to completion in its span, its duration by executable. */
    readonly run: Process["run"] = (command, options) => {
        const executable = { "process.executable.name": executableOf(command) };

        return this.#span(
            "destack.process.run",
            executable,
            () => this.source.run(command, options),
            {
                duration: { attributes: executable },
            },
        );
    };

    /** Start a child in a span lasting until it exits. */
    readonly spawn: Process["spawn"] = (command, options) =>
        this.#children(command, this.source.spawn(command, options));

    /** Start a child with a channel in a span lasting until it exits. */
    readonly fork: Process["fork"] = (command, options) =>
        this.#children(command, this.source.fork(command, options));

    /** The arguments after the executable and the script. */
    get arguments(): Process["arguments"] {
        return this.source.arguments;
    }

    /** The environment variables. */
    get variables(): Process["variables"] {
        return this.source.variables;
    }

    /** The working directory. */
    get workingDirectory(): string {
        return this.source.workingDirectory;
    }

    /** The process's identifier. */
    get id(): number {
        return this.source.id;
    }

    /** The user running the process. */
    get userId(): number | undefined {
        return this.source.userId;
    }

    /** The executable running the process. */
    get executable(): string {
        return this.source.executable;
    }

    /** The script the executable runs. */
    get script(): string {
        return this.source.script;
    }

    /** The runtime. */
    get runtime(): Process["runtime"] {
        return this.source.runtime;
    }

    /** The module loader. */
    get loader(): Process["loader"] {
        return this.source.loader;
    }

    /** The standard input. */
    get stdin(): Process["stdin"] {
        return this.source.stdin;
    }

    /** The standard output. */
    get stdout(): Process["stdout"] {
        return this.source.stdout;
    }

    /** The standard error. */
    get stderr(): Process["stderr"] {
        return this.source.stderr;
    }

    /** The channel to the parent process. */
    get parent(): Process["parent"] {
        return this.source.parent;
    }

    /** The exit code the process ends with once its work settles. */
    get exitCode(): number | undefined {
        return this.source.exitCode;
    }

    /** Set the exit code the process ends with. */
    set exitCode(code: number | undefined) {
        this.source.exitCode = code;
    }

    /** Find a command on the path. */
    readonly which: Process["which"] = (command) => this.source.which(command);

    /** Report whether a library is installed. */
    readonly hasLibrary: Process["hasLibrary"] = (name) => this.source.hasLibrary(name);

    /** Listen for a signal. */
    readonly on: Process["on"] = (signal, listener) => this.source.on(signal, listener);

    /** Send a signal to a process. */
    readonly kill: Process["kill"] = (id, signal) => this.source.kill(id, signal);

    /** Listen for failures nothing caught. */
    readonly onFailure: Process["onFailure"] = (listener) => this.source.onFailure(listener);

    /** Read the process's CPU time. */
    readonly usage: Process["usage"] = () => this.source.usage();

    /** End the process. */
    readonly exit: Process["exit"] = (code) => this.source.exit(code);
}

/** Build the tracer of children: a span from spawn to exit, its duration by executable. */
function childrenOf(telemetry: Telemetry) {
    // declare the children's duration once
    const { context, clock, monotonic } = telemetry;
    const tracer = telemetry.tracers.getTracer(ENVIRONMENT_SCOPE);
    const durations = telemetry.meters
        .getMeter(ENVIRONMENT_SCOPE)
        .createHistogram("destack.process.child.duration", {
            unit: "s",
            advice: { explicitBucketBoundaries: DURATION_BUCKETS },
        });

    return <Child extends Subprocess>(command: readonly string[], child: Child): Child => {
        // start the child's span below the active one
        const executable = { "process.executable.name": executableOf(command) };
        const started = monotonic.now();
        const parent = context.active();
        const attributes = { ...executable, "process.pid": child.id };
        const span = tracer.startSpan(
            "destack.process.child",
            { kind: SpanKind.INTERNAL, attributes, startTime: clock.now() },
            parent,
        );

        // end it with the exit code once the child exits, failing a nonzero one
        void child.exited.then((code) => {
            // describe the exit, failing a nonzero code
            const failed = code === 0 ? {} : { "error.type": String(code) };
            span.setAttributes({ "process.exit.code": code, ...failed });
            if (code !== 0) {
                span.setStatus({ code: SpanStatusCode.ERROR });
            }
            span.end(clock.now());
            const seconds = (monotonic.now() - started) / MILLISECONDS_PER_SECOND;
            durations.record(seconds, { ...executable, ...failed });
        });

        return child;
    };
}

/** Read a command's executable by its base name, as bounded as the commands a codebase runs. */
function executableOf(command: readonly string[]): string {
    const [path = ""] = command;

    return path.split("/").at(-1) ?? path;
}
