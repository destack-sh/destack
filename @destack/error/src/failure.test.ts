import { expect, test } from "vitest";
import { CallErrorCode, type DomainError } from "./error.ts";
import { Failure, FailureReason } from "./failure.ts";

/** A domain failure whose code differs from the call error it converts to. */
class NoteError extends Error implements DomainError {
    /** The note's callers, who receive the failure. */
    readonly audience = "caller";

    /** Convert the failure to the call error a caller receives. */
    toCallError(): Failure {
        return {
            code: "NOT_FOUND",
            reason: "NOTE_NOT_FOUND",
            domain: "@destack/note",
            message: this.message,
            data: { note: "note-1" },
        };
    }
}

test("describe a domain error as its call error, a call error by its code and reason, a refused connection as unavailable, a status by its code, and anything else as internal", () => {
    const coded = Object.assign(new Error("moved"), { code: "MISDIRECTED_REQUEST", data: [1] });
    const reasoned = Object.assign(new Error("taken"), {
        code: "CONFLICT",
        reason: "NAME_EXISTS",
        domain: "@destack/note",
    });
    const named = Object.assign(new Error("closed"), { code: "ECONNREFUSED" });
    const answered = Object.assign(new Error("down"), { status: 503 });

    expect([
        Failure.of(new NoteError("note note-1 not found")),
        Failure.of(coded),
        Failure.of(reasoned),
        Failure.of(named),
        Failure.of(answered),
        Failure.of("text"),
    ]).toEqual([
        {
            code: "NOT_FOUND",
            reason: "NOTE_NOT_FOUND",
            domain: "@destack/note",
            message: "note note-1 not found",
            data: { note: "note-1" },
        },
        {
            code: "MISDIRECTED_REQUEST",
            reason: "MISDIRECTED_REQUEST",
            domain: "http",
            message: "moved",
            data: [1],
        },
        { code: "CONFLICT", reason: "NAME_EXISTS", domain: "@destack/note", message: "taken" },
        {
            code: "SERVICE_UNAVAILABLE",
            reason: "SERVICE_UNAVAILABLE",
            domain: "http",
            message: "closed",
        },
        {
            code: "SERVICE_UNAVAILABLE",
            reason: "SERVICE_UNAVAILABLE",
            domain: "http",
            message: "down",
        },
        {
            code: "INTERNAL_SERVER_ERROR",
            reason: "INTERNAL_SERVER_ERROR",
            domain: "http",
            message: "text",
        },
    ]);
});

test("roundtrip a failure through the error it raises, with its cause", () => {
    const failure: Failure = {
        code: "CONFLICT",
        reason: "NAME_EXISTS",
        domain: "@destack/note",
        message: "taken",
        data: [1],
    };
    const cause = new Error("unique");
    const raised = Failure.error(failure, { cause });

    expect([Failure.of(raised), raised.cause, Failure.status(failure)]).toEqual([
        failure,
        cause,
        409,
    ]);
});

test("read a response's failure from its body, one without a reason as HTTP's, else from its status with its text", async () => {
    const reasoned = Response.json(
        { code: "GONE", reason: "NOTE_MOVED", domain: "@destack/note", message: "moved away" },
        { status: 410 },
    );
    const answered = Response.json({ code: "GONE", message: "moved away" }, { status: 410 });
    const proxied = new Response("bad gateway", { status: 502 });
    const unknown = new Response("teapot", { status: 418 });

    expect([
        await Failure.read(reasoned),
        await Failure.read(answered),
        await Failure.read(proxied),
        await Failure.read(unknown),
    ]).toEqual([
        { code: "GONE", reason: "NOTE_MOVED", domain: "@destack/note", message: "moved away" },
        { code: "GONE", reason: "GONE", domain: "http", message: "moved away" },
        { code: "BAD_GATEWAY", reason: "BAD_GATEWAY", domain: "http", message: "bad gateway" },
        { code: "BAD_REQUEST", reason: "BAD_REQUEST", domain: "http", message: "teapot" },
    ]);
});

test("read a suppressed failure as the original, naming the later one, and list its causes", () => {
    // suppress a declared and an undeclared failure with a failed disposal
    const closing = new Error("sandbox gone");
    const later = new Error("compilers did not close", { cause: closing });
    const declared = new SuppressedError(later, new NoteError("note-1 not found"));
    const cause = new Error("exited");
    const internal = new SuppressedError(later, new Error("compiler failed", { cause }));

    // keep a declared original as callers read it, name the later failure on an internal one, and list both chains
    expect([Failure.of(declared), Failure.of(internal), Failure.causes(internal)]).toEqual([
        {
            code: "NOT_FOUND",
            reason: "NOTE_NOT_FOUND",
            domain: "@destack/note",
            message: "note-1 not found",
            data: { note: "note-1" },
        },
        Failure.http("INTERNAL_SERVER_ERROR", "compiler failed, then 'compilers did not close'"),
        [internal, internal.suppressed, cause, later, closing],
    ]);
});

test("list an aggregate's errors as its causes, each with its own", () => {
    // gather two failures, one with a cause of its own
    const cause = new Error("disk full");
    const first = new Error("first close failed", { cause });
    const second = new Error("second close failed");
    const gathered = new AggregateError([first, second], "closing failed");

    // list the aggregate, then each error with its causes in order
    expect(Failure.causes(gathered)).toEqual([gathered, first, cause, second]);
});

test("read each status's HTTP reason code, else its class's", () => {
    expect([401, 402, 403, 409, 410, 507].map((status) => CallErrorCode.of(status))).toEqual([
        "UNAUTHORIZED",
        "PAYMENT_REQUIRED",
        "FORBIDDEN",
        "CONFLICT",
        "GONE",
        "INTERNAL_SERVER_ERROR",
    ]);
});

test("read a refused connection as the peer unavailable, and that as transient", () => {
    const refused = Failure.of(
        Object.assign(new Error("connect ECONNREFUSED"), { code: "ECONNREFUSED" }),
    );

    expect([refused, Failure.isTransient(refused)]).toEqual([
        {
            code: "SERVICE_UNAVAILABLE",
            reason: "SERVICE_UNAVAILABLE",
            domain: "http",
            message: "connect ECONNREFUSED",
        },
        true,
    ]);
});

test("declare domain, call, connection and status errors, and leave programming and internal errors undeclared", () => {
    const closed = Object.assign(new TypeError("the socket closed"), { code: "ConnectionClosed" });
    const reset = Object.assign(new Error("reset"), { code: "ECONNRESET" });
    const status = Object.assign(new Error("teapot"), { status: 418 });
    const call = Object.assign(new Error("gone"), { code: "NOT_FOUND" });
    const internal = Object.assign(new Error("broken"), { code: "INTERNAL_SERVER_ERROR" });
    const failed = Object.assign(new Error("broken"), { status: 500 });

    expect(
        [
            new NoteError("missing"),
            call,
            closed,
            reset,
            status,
            internal,
            failed,
            new TypeError("x is undefined"),
            "text",
        ].map((error) => Failure.isDeclared(error)),
    ).toEqual([true, true, true, true, true, false, false, false, false]);
});

test("refuse for good with a declared failure no retry fixes, and leave lost, transient and undeclared failures to a retry", () => {
    const forbidden = Object.assign(new Error("no"), { status: 403 });
    const unavailable = Object.assign(new Error("starting"), { status: 503 });
    const reset = Object.assign(new Error("reset"), { code: "ECONNRESET" });
    const lost = new Error("network connection lost");

    expect(
        [new NoteError("missing"), forbidden, unavailable, reset, lost, "text"].map((error) =>
            Failure.isRefusal(error),
        ),
    ).toEqual([true, true, false, false, false, false]);
});

test("describe a failure without a package's reason as HTTP's, its code its reason", () => {
    expect(Failure.http("NOT_FOUND", "no note")).toEqual({
        code: "NOT_FOUND",
        reason: "NOT_FOUND",
        domain: "http",
        message: "no note",
    });
});

test("accept reasons of constant-case words of at most 63 characters only", () => {
    const reasons = ["NAME_EXISTS", "A", "name_taken", "NAME_", "N".repeat(63), "N".repeat(64)];

    expect(reasons.map((reason) => FailureReason.safeParse(reason).success)).toEqual([
        true,
        false,
        false,
        false,
        true,
        false,
    ]);
});

test("retry a lost race, a move, an outage and a limit that says when, and nothing else", () => {
    const reasons = [
        "REVISION_CONFLICT",
        "SCOPE_MOVED",
        "SERVER_UNAVAILABLE",
        "STORAGE_EXHAUSTED",
        "RESPONSE_UNREACHABLE",
        "BUILD_BLOCKED",
        "NOTE_NOT_FOUND",
        "RECORD_BROKEN",
    ];
    const limited = {
        code: "TOO_MANY_REQUESTS",
        reason: "RATE_EXHAUSTED",
        data: { retryAfter: 1000 },
    } as const;

    expect([
        ...reasons.map((reason) => Failure.isTransient({ code: "BAD_REQUEST", reason })),
        Failure.isTransient(limited),
    ]).toEqual([true, true, true, false, false, false, false, false, true]);
});
