import { expect, test } from "@destack/test";
import { BasicCredential } from "./basic.ts";

test("write a user and a password as a Basic header and read them back, a password with colons whole", () => {
    const header = BasicCredential.write("ada", "pass:word");

    expect([
        header,
        BasicCredential.read(header),
        BasicCredential.read(header.replace("Basic", "basic")),
        BasicCredential.is(header),
        BasicCredential.challenge("destack"),
    ]).toEqual([
        "Basic YWRhOnBhc3M6d29yZA==",
        { username: "ada", password: "pass:word" },
        { username: "ada", password: "pass:word" },
        true,
        'Basic realm="destack"',
    ]);
});

test("read nothing from another scheme, malformed base64 or a pair without a colon", () => {
    const pairless = `Basic ${new TextEncoder().encode("ada").toBase64()}`;

    expect([
        BasicCredential.read("Bearer token"),
        BasicCredential.read("Basic not base64!"),
        BasicCredential.read(pairless),
        BasicCredential.read(null),
        BasicCredential.is("Bearer token"),
    ]).toEqual([undefined, undefined, undefined, undefined, false]);
});
