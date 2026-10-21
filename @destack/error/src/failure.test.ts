import { expect, test } from "vitest";
import { CallErrorCode, type DomainError } from "./error.ts";
import { Failure } from "./failure.ts";

/** A domain failure whose code differs from the call error it converts to. */
class NoteError extends Error implements DomainError {
    /** Convert the failure to the call error a caller receives. */
    toCallError(): Failure {
        return { code: "NOT_FOUND", message: this.message, data: { note: "note-1" } };
    }
}

test("describe a domain error as its call error, a call error by its code, a status by its code, and anything else as internal", () => {
    const coded = Object.assign(new Error("moved"), { code: "MISDIRECTED_REQUEST", data: [1] });
    const named = Object.assign(new Error("closed"), { code: "ECONNREFUSED" });
    const answered = Object.assign(new Error("down"), { status: 503 });

    expect([
        Failure.of(new NoteError("no note note-1")),
        Failure.of(coded),
        Failure.of(named),
        Failure.of(answered),
        Failure.of("text"),
    ]).toEqual([
        { code: "NOT_FOUND", message: "no note note-1", data: { note: "note-1" } },
        { code: "MISDIRECTED_REQUEST", message: "moved", data: [1] },
        { code: "INTERNAL_SERVER_ERROR", message: "closed" },
        { code: "SERVICE_UNAVAILABLE", message: "down" },
        { code: "INTERNAL_SERVER_ERROR", message: "text" },
    ]);
});

test("roundtrip a failure through the error it raises, with its cause", () => {
    const failure: Failure = { code: "CONFLICT", message: "taken", data: [1] };
    const cause = new Error("unique");
    const raised = Failure.error(failure, { cause });

    expect([Failure.of(raised), raised.cause, Failure.status(failure)]).toEqual([
        failure,
        cause,
        409,
    ]);
});

test("read a response's failure from its body, else from its status with its text", async () => {
    const answered = Response.json({ code: "GONE", message: "moved away" }, { status: 410 });
    const proxied = new Response("bad gateway", { status: 502 });
    const unknown = new Response("teapot", { status: 418 });

    expect([
        await Failure.read(answered),
        await Failure.read(proxied),
        await Failure.read(unknown),
    ]).toEqual([
        { code: "GONE", message: "moved away" },
        { code: "BAD_GATEWAY", message: "bad gateway" },
        { code: "BAD_REQUEST", message: "teapot" },
    ]);
});

test("read a status's HTTP reason code before Destack's own codes of the status", () => {
    expect([401, 403, 409, 410, 507].map((status) => CallErrorCode.of(status))).toEqual([
        "UNAUTHORIZED",
        "FORBIDDEN",
        "CONFLICT",
        "GONE",
        "INTERNAL_SERVER_ERROR",
    ]);
});
