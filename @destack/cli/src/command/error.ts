import { CallError } from "@destack/object";
import type { Terminal } from "../output/index.ts";
import { CliError } from "../error/error.ts";
import type { CliEnvironment } from "../context/context.ts";

/** The exit status of a command that failed other than by its usage, as POSIX utilities exit. */
const FAILURE_STATUS = 1;

/** The exit status of a command misused, as POSIX utilities and Bash's builtins exit. */
const USAGE_STATUS = 2;

/** The exit status of a command needing a person signed in. */
const SIGN_IN_STATUS = 4;

/** A command the terminal's interrupt stopped, which exits as an interrupted program does. */
export class Interrupted extends Error {
    /** The exit status of a program SIGINT stopped: 128 plus the signal's number, 2. */
    static readonly STATUS = 130;

    /** Stop a command at the terminal's interrupt. */
    constructor() {
        super("interrupted");
        this.name = "Interrupted";
    }
}

/** The interrupts of a running command: the first aborts it, a second stops its cleanup. */
export class Interrupt implements Disposable {
    /** Aborted at the first interrupt, with `Interrupted` as its reason. */
    readonly signal: AbortSignal;
    /** Resolved at a second interrupt, which stops waiting for cleanup. */
    readonly again: Promise<void>;
    /** The listener of the process's interrupts. */
    readonly #listening: Disposable;

    /** Listen to a process's interrupts until disposed. */
    constructor(process: Pick<CliEnvironment["process"], "on">) {
        // abort at the first interrupt, resolve the second
        const controller = new AbortController();
        const { promise, resolve } = Promise.withResolvers<void>();
        this.signal = controller.signal;
        this.again = promise;
        this.#listening = process.on("SIGINT", () => {
            if (controller.signal.aborted) {
                resolve();
            } else {
                controller.abort(new Interrupted());
            }
        });
    }

    /** Stop a command the interrupt ended, which exits as interrupted. */
    require(): void {
        if (this.signal.aborted) {
            throw new Interrupted();
        }
    }

    /** Stop listening. */
    [Symbol.dispose](): void {
        this.#listening[Symbol.dispose]();
    }
}

/** A command's failure as the terminal reports it: its code, message and exit status. */
export class CommandFailure {
    /** The failure. */
    readonly error: unknown;
    /** The first failure with a code in the error's cause chain, absent for an uncoded one. */
    readonly cause: Error | undefined;

    /** Read a failure and its first coded cause. */
    constructor(error: unknown) {
        // find the first failure with a code, which the HTTP client wraps
        this.error = error;
        let cause = error;
        while (cause instanceof Error && CallError.of(cause) === undefined) {
            cause = cause.cause;
        }
        this.cause = cause instanceof Error ? cause : undefined;
    }

    /** Read the code the failure reports in the call error vocabulary. */
    get code(): string {
        return CallError.of(this.cause)?.code ?? "INTERNAL_SERVER_ERROR";
    }

    /** Read the message of the coded failure, else of the failure itself. */
    get message(): string {
        const failure = this.cause ?? this.error;

        return failure instanceof Error ? failure.message : String(failure);
    }

    /** Read the exit status: misuse, a missing sign-in, an interrupt, or any other failure. */
    get status(): number {
        // exit as interrupted
        if (this.error instanceof Interrupted) {
            return Interrupted.STATUS;
        }
        // exit as misused, such as without a space to act in
        else if (
            CliError.is(this.cause, "USAGE_INVALID") ||
            CliError.is(this.cause, "SPACE_REQUIRED")
        ) {
            return USAGE_STATUS;
        }
        // exit as a command needing a sign-in, refused locally or by a service
        else if (
            CliError.is(this.cause, "SIGN_IN_REQUIRED") ||
            CallError.of(this.cause)?.code === "UNAUTHORIZED"
        ) {
            return SIGN_IN_STATUS;
        }
        // exit as any other failure
        else {
            return FAILURE_STATUS;
        }
    }

    /** Report the failure, quietly for an interrupt, and return its exit status. */
    async report(json: boolean, terminal: Terminal): Promise<number> {
        if (!(this.error instanceof Interrupted)) {
            const reported = { code: this.code, message: this.message };
            await terminal.printDiagnostic(
                json ? JSON.stringify({ error: reported }) : `destack: ${reported.message}`,
            );
        }

        return this.status;
    }
}
