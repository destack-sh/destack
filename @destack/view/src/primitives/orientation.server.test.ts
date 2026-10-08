import { expect, test } from "@destack/test";
import { createOrientation } from "./orientation.ts";

test("read a portrait orientation on the server", () => {
    const { angle, type } = createOrientation();

    expect([angle(), type()]).toEqual([0, "portrait-primary"]);
});
