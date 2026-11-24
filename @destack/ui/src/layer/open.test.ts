import { expect, test } from "@destack/test";
import { controlOf } from "./index.ts";

test("take a control given alone, make one without, and refuse one beside an open state", () => {
    // ask with a control, without, and with each property of an open state beside it
    const given = { name: "given" };
    const made = { name: "made" };
    const refused = [{ open: false }, { defaultOpen: true }, { onOpenChange: () => undefined }].map(
        (properties) => () => controlOf({ control: given, ...properties }, () => made),
    );

    expect([
        controlOf({ control: given }, () => made),
        controlOf({ open: true }, () => made),
    ]).toEqual([given, made]);
    for (const refuse of refused) {
        expect(refuse).toThrow(
            new TypeError("an overlay given a control leaves its open state to it"),
        );
    }
});
