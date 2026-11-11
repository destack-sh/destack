import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { StackTarget } from "./reference.ts";

test("read a stack target tagged on a field, before or after it is made optional", () => {
    const text = schema.string();

    expect([
        StackTarget.of(StackTarget.on(text, () => "subject")),
        StackTarget.of(StackTarget.on(text, () => "object").exactOptional()),
        StackTarget.of(StackTarget.on(text.exactOptional(), () => "subject")),
        StackTarget.of(text),
    ]).toEqual(["subject", "object", "subject", undefined]);
});
