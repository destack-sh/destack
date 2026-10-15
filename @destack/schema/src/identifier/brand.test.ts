import { expect, test } from "@destack/test";
import { z } from "zod";
import { brand } from "./brand.ts";

test("brand a validator in types alone, keeping what it accepts and refuses", () => {
    // validate the same values as the unbranded validator does
    const validator = z.string().regex(/^note-\d+$/u);
    const branded = brand<"note">(validator);

    expect({
        isSame: branded === validator,
        accepted: branded.safeParse("note-1").success,
        refused: branded.safeParse("page-1").success,
    }).toEqual({ isSame: true, accepted: true, refused: false });
});
