import { createORPCErrorFromJson, isORPCErrorJson } from "@orpc/client";
import { expect, test } from "@destack/test";
import { isServiceError, ServiceError } from "./error.ts";

test("recognise a service error decoded from a response and refuse a plain error", () => {
    // decode the body a client reads from a refused call
    const sent = new ServiceError("MISDIRECTED_REQUEST", {
        message: "moved",
        data: { machine: "machine-1" },
    });
    const body: unknown = JSON.parse(JSON.stringify(sent.toJSON()));
    const decoded = isORPCErrorJson(body) ? createORPCErrorFromJson(body) : body;
    expect([
        isServiceError(decoded),
        isServiceError(decoded) && [decoded.code, decoded.status, decoded.data],
        isServiceError(new Error("moved")),
    ]).toEqual([true, ["MISDIRECTED_REQUEST", 421, { machine: "machine-1" }], false]);
});

test("read a misdirected answer's declared data from a failure and a response, and nothing from another answer", async () => {
    // misdirect to a machine and to a partition, and answer otherwise
    const moved = new ServiceError("MISDIRECTED_REQUEST", {
        message: "space-1 moves to machine-2",
        data: { scope: "space-1", machine: "machine-2" },
    });
    const partitioned = new ServiceError("MISDIRECTED_REQUEST", {
        message: "room-1 is served by partition room-1",
        data: { scope: "room-1", partition: "room-1" },
    });
    expect([
        ServiceError.data(moved, "MISDIRECTED_REQUEST"),
        await ServiceError.read(respond(partitioned), "MISDIRECTED_REQUEST"),
        ServiceError.data(
            new ServiceError("NOT_FOUND", { message: "no space-1" }),
            "MISDIRECTED_REQUEST",
        ),
        await ServiceError.read(
            new Response("misdirected", { status: 421 }),
            "MISDIRECTED_REQUEST",
        ),
    ]).toEqual([
        { scope: "space-1", machine: "machine-2" },
        { scope: "room-1", partition: "room-1" },
        undefined,
        undefined,
    ]);
});

test("follow a misdirected answer once, handing the second attempt where its scope is served", async () => {
    // misdirect every attempt to another machine
    const attempts: unknown[] = [];
    const answered = await ServiceError.follow(async (misdirected) => {
        attempts.push(misdirected);
        const error = new ServiceError("MISDIRECTED_REQUEST", {
            message: "space-1 moves to machine-2",
            data: { scope: "space-1", machine: "machine-2" },
        });

        return Response.json(error.toJSON(), { status: error.status });
    });

    expect([answered.status, attempts]).toEqual([
        421,
        [undefined, { scope: "space-1", machine: "machine-2" }],
    ]);
});

/** Answer a failure as a service answers it. */
function respond(error: ServiceError<string, unknown>): Response {
    return Response.json(error.toJSON(), { status: error.status });
}
