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
