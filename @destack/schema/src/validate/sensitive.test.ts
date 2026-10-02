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
