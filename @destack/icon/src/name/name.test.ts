import { expect, test } from "@destack/test";
import acorn from "../phosphor/icon/acorn.ts";
import { ICON_NAMES } from "../phosphor/names.ts";
import { IconBodies } from "../icon/icon.ts";
import { IconName } from "./name.ts";

test("accept a Phosphor icon's name and refuse another, and load an icon's bodies by name", async () => {
    // validate a name of the set and one outside it, and load the acorn's bodies
    const loaded = await IconBodies.load(IconName.parse("acorn"));

    expect({
        accepted: IconName.safeParse("book-open").success,
        refused: IconName.safeParse("book-closed-shut").success,
        first: ICON_NAMES[0],
        loaded: loaded === acorn,
    }).toEqual({ accepted: true, refused: false, first: "acorn", loaded: true });
});
