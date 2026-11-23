import { Failure } from "@destack/error";
import { schema } from "@destack/schema";
import type { Terminal } from "../terminal/index.ts";
import { CliError } from "../error/error.ts";
import type { CommandEnvironment } from "./context.ts";

/** The exit status of a command that failed other than by its usage, as POSIX utilities exit. */
const FAILURE_STATUS = 1;

/** The exit status of a command misused, as POSIX utilities and Bash's builtins exit. */
const USAGE_STATUS = 2;

/** The exit status of a command needing a person signed in. */
const SIGN_IN_STATUS = 4;

/** The exit status of a program SIGINT stopped: 128 plus the signal's number, 2. */
const INTERRUPTED_STATUS = 130;

/** The interrupts of a running command: the first aborts it, a second stops its cleanup. */
export class Interrupt implements Disposable {
    /** Aborted at the first interrupt, with the cancelled command as its reason. */
    readonly signal: AbortSignal;
    /** Resolved at a second interrupt, which stops waiting for cleanup. */
    readonly again: Promise<void>;
    /** The listener of the process's interrupts. */
    readonly #listening: Disposable;

    /** Listen to a process's interrupts until disposed. */
    constructor(process: Pick<CommandEnvironment["process"], "on">) {
        // abort at the first interrupt, resolve the second
        const controller = new AbortController();
        const { promise, resolve } = Promise.withResolvers<void>();
        this.signal = controller.signal;
        this.again = promise;
        this.#listening = process.on("SIGINT", () => {
            if (controller.signal.aborted) {
                resolve();
            } else {
                controller.abort(Interrupt.#cancelled());
            }
        });
    }

    /** Stop a command the interrupt ended, which exits as interrupted. */
    require(): void {
        if (this.signal.aborted) {
            throw Interrupt.#cancelled();
        }
    }

    /** Fail a command the interrupt cancelled. */
    static #cancelled(): Error {
        return CliError.of("COMMAND_CANCELLED", "command was cancelled");
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
    /** The call failure the coded cause declares, absent for an uncoded one. */
    readonly failure: Failure | undefined;

    /** Read a failure and its first coded cause. */
    constructor(error: unknown) {
        // find the first failure with a code, which the HTTP client wraps
        this.error = error;
        this.cause = Failure.causes(error).find((cause) => Failure.declared(cause) !== undefined);
        this.failure = Failure.declared(this.cause);
    }

    /** Read the code the failure reports in the call error vocabulary. */
    get code(): string {
        return this.failure?.code ?? "INTERNAL_SERVER_ERROR";
    }

    /** Read the message of the coded failure, else of the failure itself. */
    get message(): string {
        const failure = this.cause ?? this.error;

        return failure instanceof Error ? failure.message : String(failure);
    }

    /** Read the messages of the failures that caused the reported one, outermost first. */
    get causes(): string[] {
        // walk the cause chain below the reported failure
        const causes: string[] = [];
        const reported = this.cause ?? this.error;
        let cause = reported instanceof Error ? reported.cause : undefined;
        while (cause !== undefined) {
            causes.push(cause instanceof Error ? cause.message : JSON.stringify(cause));
            cause = cause instanceof Error ? cause.cause : undefined;
        }

        return causes;
    }

    /** Read the trace a service captured its internal failure in. */
    get trace(): string | undefined {
        const traced = schema
            .looseObject({ traceId: schema.string() })
            .safeParse(this.failure?.data);

        return traced.success ? traced.data.traceId : undefined;
    }

    /** Read the exit status: misuse, a missing sign-in, an interrupt, or any other failure. */
    get status(): number {
        // exit as interrupted
        if (CliError.is(this.error, "COMMAND_CANCELLED")) {
            return INTERRUPTED_STATUS;
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
            CliError.is(this.cause, "SIGN_IN_UNAUTHENTICATED") ||
            this.failure?.code === "UNAUTHORIZED"
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
        if (!CliError.is(this.error, "COMMAND_CANCELLED")) {
            const reported = { code: this.code, message: this.message, causes: this.causes };
            const { trace } = this;
            const lines = [
                terminal.event("error", "failed", reported.message),
                ...reported.causes.map((cause) => terminal.hint("cause", cause)),
                ...(trace === undefined ? [] : [terminal.hint("trace", trace)]),
            ];
            await terminal.printDiagnostic(
                json ? JSON.stringify({ error: reported }) : lines.join("\n"),
            );
        }

        return this.status;
    }
}
