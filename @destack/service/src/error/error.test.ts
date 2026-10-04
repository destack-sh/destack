import { createORPCErrorFromJson, isORPCErrorJson } from "@orpc/client";
import { expect, test } from "@destack/test";
import { isServiceError, ServiceError } from "./error.ts";

test("recognise a service error decoded from a response and refuse a plain error", () => {
    // decode the body a client reads from a refused call
    const sent = new ServiceError("MOVED", { message: "moved", data: { cell: "eu-1" } });
    const body: unknown = JSON.parse(JSON.stringify(sent.toJSON()));
    const decoded = isORPCErrorJson(body) ? createORPCErrorFromJson(body) : body;
    expect([
        isServiceError(decoded),
        isServiceError(decoded) && [decoded.code, decoded.status, decoded.data],
        isServiceError(new Error("moved")),
    ]).toEqual([true, ["MOVED", 421, { cell: "eu-1" }], false]);
});
