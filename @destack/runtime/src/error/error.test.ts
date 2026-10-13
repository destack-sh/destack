import { createORPCErrorFromJson, isORPCErrorJson } from "@orpc/client";
import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { isCallError, CallError } from "./error.ts";

/** The machine a misdirected answer moves the scope to. */
const MOVED = schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-0000000000e1");

test("recognise a call error decoded from a response and refuse a plain error", () => {
    // decode the body a client reads from a refused call
    const sent = new CallError("MISDIRECTED_REQUEST", {
        message: "moved",
        data: { machine: "machine-1" },
    });
    const body: unknown = JSON.parse(JSON.stringify(sent.toJSON()));
    const decoded = isORPCErrorJson(body) ? createORPCErrorFromJson(body) : body;
    expect([
        isCallError(decoded),
        isCallError(decoded) && [decoded.code, decoded.status, decoded.data],
        isCallError(new Error("moved")),
    ]).toEqual([true, ["MISDIRECTED_REQUEST", 421, { machine: "machine-1" }], false]);
});

test("read a misdirected answer's declared data from a failure and a response, and nothing from another answer", async () => {
    // misdirect to a machine and to a partition, and answer otherwise
    const moved = new CallError("MISDIRECTED_REQUEST", {
        message: `space-1 moves to ${MOVED}`,
        data: { scope: "space-1", machine: MOVED },
    });
    const partitioned = new CallError("MISDIRECTED_REQUEST", {
        message: "room-1 is served by partition room-1",
        data: { scope: "room-1", partition: "room-1" },
    });
    expect([
        CallError.data(moved, "MISDIRECTED_REQUEST"),
        await CallError.read(respond(partitioned), "MISDIRECTED_REQUEST"),
        CallError.data(
            new CallError("NOT_FOUND", { message: "no space-1" }),
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
