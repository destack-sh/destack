import { ORPCError } from "@orpc/client";
import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { CallError, findDenial } from "./call.ts";
import { ObjectError } from "./error.ts";

import type {} from "@destack/package/import-meta";

/** The machine a misdirected answer moves the scope to. */
const MOVED = schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-0000000000e1");

test("decode a call error from its body with its reason and domain, read oRPC's own failure as HTTP's, and refuse a plain error", () => {
    // decode the body a client reads from a refused call
    const sent = new CallError("MISDIRECTED_REQUEST", {
        reason: "SCOPE_MOVED",
        domain: import.meta.destack.package.name,
        message: "moved",
        data: { machine: "machine-1" },
    });
    const body: unknown = JSON.parse(JSON.stringify(sent.toJSON()));
    const decoded = CallError.fromJson(body);
    const foreign = CallError.of(
        new ORPCError("MISDIRECTED_REQUEST", {
            status: 421,
            message: "moved",
            data: { machine: "machine-1" },
        }),
    );
    expect([read(decoded), read(foreign), CallError.of(new Error("moved"))]).toEqual([
        ["MISDIRECTED_REQUEST", "SCOPE_MOVED", "@destack/object", 421, { machine: "machine-1" }],
        ["MISDIRECTED_REQUEST", "MISDIRECTED_REQUEST", "http", 421, { machine: "machine-1" }],
        undefined,
    ]);
});

test("read a misdirected answer's declared data from a failure and a response, and nothing from another answer", async () => {
    // misdirect to a machine and to a partition, and answer otherwise
    const moved = new CallError("MISDIRECTED_REQUEST", {
        reason: "SCOPE_MOVED",
        domain: import.meta.destack.package.name,
        message: `space-1 moves to ${MOVED}`,
        data: { scope: "space-1", machine: MOVED },
    });
    const partitioned = new CallError("MISDIRECTED_REQUEST", {
        reason: "PARTITION_MISDIRECTED",
        domain: import.meta.destack.package.name,
        message: "room-1 is served by partition room-1",
        data: { scope: "room-1", partition: "room-1" },
    });
    expect([
        CallError.data(moved, "MISDIRECTED_REQUEST"),
        await CallError.read(respond(partitioned), "MISDIRECTED_REQUEST"),
        CallError.data(
            CallError.http("NOT_FOUND", { message: "no space-1" }),
            "MISDIRECTED_REQUEST",
        ),
        await CallError.read(new Response("misdirected", { status: 421 }), "MISDIRECTED_REQUEST"),
    ]).toEqual([
        { scope: "space-1", machine: MOVED },
        { scope: "room-1", partition: "room-1" },
        undefined,
        undefined,
    ]);
});

test("follow a misdirected answer once, handing the second attempt where its scope is served", async () => {
    // misdirect every attempt to another machine
    const attempts: unknown[] = [];
    const answered = await CallError.follow(async (misdirected) => {
        attempts.push(misdirected);
        const error = new CallError("MISDIRECTED_REQUEST", {
            reason: "SCOPE_MOVED",
            domain: import.meta.destack.package.name,
            message: `space-1 moves to ${MOVED}`,
            data: { scope: "space-1", machine: MOVED },
        });

        return Response.json(error.toJSON(), { status: error.status });
    });

    expect([answered.status, attempts]).toEqual([
        421,
        [undefined, { scope: "space-1", machine: MOVED }],
    ]);
});

/** Answer a failure as a server answers it. */
function respond(error: CallError<string, unknown>): Response {
    return Response.json(error.toJSON(), { status: error.status });
}

/** Read a call error's code, reason, domain, status and data. */
function read(error: CallError<string, unknown> | undefined) {
    return error === undefined
        ? undefined
        : [error.code, error.reason, error.domain, error.status, error.data];
}

test("find the denial a missing scope conceals, through a family failure, and none behind a plain missing one", () => {
    // conceal a refusal behind the scope's absence, and fail with a missing scope alone
    const denial = ObjectError.of("PERMISSION_DENIED", "permission denied: read");
    const concealed = ObjectError.of("SCOPE_NOT_FOUND", "scope space-1 not found", {
        cause: denial,
    });
    const missing = ObjectError.of("SCOPE_NOT_FOUND", "scope space-2 not found");

    // read the concealed refusal as the denial, and nothing for the plain missing scope
    const found = CallError.of(findDenial(concealed));
    expect([
        found === undefined ? undefined : [found.code, found.message],
        findDenial(missing),
    ]).toEqual([["FORBIDDEN", "permission denied: read"], undefined]);
});
