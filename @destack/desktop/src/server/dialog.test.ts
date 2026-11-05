import { GrantPermission } from "@destack/access";
import { Localization, SOURCE_LOCALE } from "@destack/locale";
import { expect, test } from "@destack/test";
import { recordShell } from "../test/index.ts";
import { Dialog } from "./dialog.ts";

/** The package declaring the permissions. */
const NOTES_PACKAGE = "package-01996ab0-0000-7000-8000-000000000001";

test("ask whether a view may act for the person, listing each permission once under the installation", async () => {
    // ask about reading notes in two scopes and writing them
    const { shell, sent } = recordShell();
    const permission = (name: string, scope?: string) =>
        GrantPermission.schema.parse({
            packageId: NOTES_PACKAGE,
            type: "note",
            name,
            ...(scope === undefined ? {} : { scope }),
        });
    const address = {
        space: "personal--florian",
        installation: "notes",
        view: "home",
        path: "/",
    };
    await new Dialog("window-1", address).ask(shell, Localization.of(SOURCE_LOCALE, []), [
        permission("read", "space-1"),
        permission("read", "space-2"),
        permission("write"),
    ]);

    expect(sent).toEqual([
        {
            action: "dialog.ask",
            id: "window-1",
            title: "Allow ⁨notes⁩ to act for you?",
            body: "read note\nwrite note",
            allow: "Allow",
            deny: "Don’t Allow",
        },
    ]);
});
