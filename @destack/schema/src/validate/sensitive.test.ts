import { expect, test } from "@destack/test";
import * as schema from "./index.ts";

test("leave out sensitive values through objects, arrays and wrappers, handing each to the finder", () => {
    // mark a password and a list of tokens sensitive
    const login = schema.object({
        name: schema.string(),
        password: schema.sensitive(schema.string()),
        tokens: schema.array(schema.sensitive(schema.string())).exactOptional(),
        recovery: schema.nullable(schema.sensitive(schema.string())),
    });

    // drop each sensitive value, keeping array positions as null
    const found: unknown[] = [];
    expect(
        schema.redact(
            login,
            { name: "ada", password: "secret", tokens: ["a", "b"], recovery: null },
            (value) => found.push(value),
        ),
    ).toEqual({ name: "ada", tokens: [null, null], recovery: null });
    expect(found).toEqual(["secret", "a", "b"]);
});

test("map personal values apart from secrets, through the same walk that leaves secrets out", () => {
    // mark an address personal and a token secret
    const call = schema.object({
        address: schema.sensitive(schema.string(), "personal"),
        token: schema.sensitive(schema.string()),
        method: schema.string(),
    });

    // seal the personal value and drop the secret
    expect([
        schema.sensitivityOf(call.shape.address),
        schema.mapSensitive(
            call,
            { address: "ada@example.com", token: "t", method: "update" },
            (value, sensitivity) => (sensitivity === "personal" ? { sealed: value } : undefined),
        ),
        schema.redact(call, { address: "ada@example.com", token: "t", method: "update" }),
    ]).toEqual([
        "personal",
        { address: { sealed: "ada@example.com" }, method: "update" },
        { method: "update" },
    ]);
});

test("mark a copy of a shared schema, leaving the original and its other uses plain", () => {
    // mark a shared schema personal
    const attributes = schema.record(schema.string(), schema.string());
    const personal = schema.sensitive(attributes, "personal");

    // read the mark on the copy, its clones and the original
    expect([
        personal === attributes,
        schema.sensitivityOf(personal),
        schema.isSensitive(personal),
        schema.sensitivityOf(personal.describe("the personal attributes")),
        schema.isSensitive(personal.describe("the personal attributes")),
        schema.sensitivityOf(attributes),
        schema.isSensitive(attributes),
    ]).toEqual([false, "personal", true, "personal", true, undefined, false]);

    // keep values under the original and drop them under the copy
    const value = { "enduser.email": "ada@example.com" };
    expect([schema.redact(attributes, value), schema.redact(personal, value)]).toEqual([
        value,
        undefined,
    ]);
});
