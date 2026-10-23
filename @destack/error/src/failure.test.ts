import { expect, test } from "vitest";
import { CallErrorCode, type DomainError } from "./error.ts";
import { Failure, FailureReason } from "./failure.ts";

/** A domain failure whose code differs from the call error it converts to. */
class NoteError extends Error implements DomainError {
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
        reason: "NAME_TAKEN",
        domain: "@destack/note",
    });
    const named = Object.assign(new Error("closed"), { code: "ECONNREFUSED" });
    const answered = Object.assign(new Error("down"), { status: 503 });

    expect([
        Failure.of(new NoteError("no note note-1")),
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
            message: "no note note-1",
            data: { note: "note-1" },
        },
        {
            code: "MISDIRECTED_REQUEST",
            reason: "MISDIRECTED_REQUEST",
            domain: "http",
            message: "moved",
            data: [1],
        },
        { code: "CONFLICT", reason: "NAME_TAKEN", domain: "@destack/note", message: "taken" },
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
        reason: "NAME_TAKEN",
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

test("describe a failure without a package's reason as HTTP's, its code its reason", () => {
    expect(Failure.http("NOT_FOUND", "no note")).toEqual({
        code: "NOT_FOUND",
        reason: "NOT_FOUND",
        domain: "http",
        message: "no note",
    });
});

test("accept reasons of constant-case words of at most 63 characters only", () => {
    const reasons = ["NAME_TAKEN", "A", "name_taken", "NAME_", "N".repeat(63), "N".repeat(64)];

    expect(reasons.map((reason) => FailureReason.safeParse(reason).success)).toEqual([
        true,
        false,
        false,
        false,
        true,
        false,
    ]);
});
