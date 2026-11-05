import { expect, test } from "@destack/test";
import { recordShell } from "../test/index.ts";
import { Badge } from "./badge.ts";

test("show a count on the badge, clearing it at zero and once nobody is signed in", async () => {
    // show three, then zero, then clear
    const { shell, sent } = recordShell();
    const badge = new Badge(shell);
    await badge.show(3);
    await badge.show(0);
    await badge.show(1);
    await badge.clear();

    expect(sent).toEqual([
        { action: "badge.set", count: 3 },
        { action: "badge.set", count: null },
        { action: "badge.set", count: 1 },
        { action: "badge.set", count: null },
    ]);
});
