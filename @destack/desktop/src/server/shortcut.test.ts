import { expect, test } from "@destack/test";
import { recordShell } from "../test/index.ts";
import { Shortcut } from "./shortcut.ts";

test("register keys only once they change, and again after the shortcut is released", async () => {
    // register the same keys twice, change them, release them, then register the same keys again
    const { shell, sent } = recordShell();
    const shortcut = new Shortcut("palette", shell);
    await shortcut.register("alt+space");
    await shortcut.register("alt+space");
    await shortcut.register("mod+k");
    await shortcut.clear();
    await shortcut.register("mod+k");

    expect(sent).toEqual([
        { action: "shortcut.register", id: "palette", shortcut: "alt+space" },
        { action: "shortcut.register", id: "palette", shortcut: "mod+k" },
        { action: "shortcut.unregister", id: "palette" },
        { action: "shortcut.register", id: "palette", shortcut: "mod+k" },
    ]);
});
