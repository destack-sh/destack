import { Localization, SOURCE_LOCALE, t } from "@destack/locale";
import { defineCommand } from "@destack/object";
import { testSimulation } from "@destack/object/test";
import { expect, refusal, test } from "@destack/test";
import { window } from "../object/index.ts";
import { desktopObjects, recordShell } from "../test/index.ts";
import { Menu, type MenuEntry } from "./menu.ts";

/** The account section while nobody is signed in. */
const SIGN_IN: MenuEntry = { kind: "action", id: "signIn", title: "Sign In…", run: async () => {} };

/** The update section while the release is current. */
const CHECK: MenuEntry = {
    kind: "action",
    id: "check",
    title: "Check for Updates…",
    run: async () => {},
};

/** The serving section's status line. */
const STATUS: MenuEntry = { kind: "label", title: "Serving 1 space" };

test("fill each menu with its sections in order, separating the non-empty groups", async () => {
    const { shell, sent } = recordShell();
    const menu = new Menu(shell, Localization.of(SOURCE_LOCALE, []));

    // fill the account, the update and the serving sections, then empty the update section
    await menu.set("account", [SIGN_IN]);
    await menu.set("update", [CHECK]);
    await menu.set("serving", [STATUS]);
    await menu.set("update", []);

    // expect each change to resend the menus showing the section, each item by section and identity
    const signIn = { kind: "action", id: "account.signIn", title: "Sign In…" };
    const check = { kind: "action", id: "update.check", title: "Check for Updates…" };
    const status = { kind: "label", id: "serving.label.0", title: "Serving 1 space" };
    const separator = { kind: "separator" };
    expect(sent).toEqual([
        { action: "menu.set", menu: "tray", items: [signIn] },
        { action: "menu.set", menu: "tray", items: [check, separator, signIn] },
        { action: "menu.set", menu: "application", items: [check] },
        { action: "menu.set", menu: "tray", items: [status, separator, check, separator, signIn] },
        { action: "menu.set", menu: "tray", items: [status, separator, signIn] },
        { action: "menu.set", menu: "application", items: [] },
    ]);
});

test("offer a command only on targets meeting its condition, and run it on the one chosen", async () => {
    // open a view's window and a palette, and declare a command focusing views
    const objects = await desktopObjects(testSimulation().environment("desktop"));
    const view = await objects
        .mutate(window)
        .open({ link: "destack://notes--personal--florian/home" }).predicted;
    const palette = await objects
        .mutate(window)
        .open({ link: "destack://home--personal--florian/command", kind: "palette" }).predicted;
    const focus = defineCommand(
        {
            name: "focus-view",
            title: t`Focus`,
            object: window,
            method: "focus",
            when: { kind: "view" },
        },
        { package: window.package },
    );

    // offer it on both windows, then choose the view's
    const { shell, sent } = recordShell();
    const menu = new Menu(shell, Localization.of(SOURCE_LOCALE, []));
    const offered = [
        ...(await menu.offer({ command: focus }, objects, view.id)),
        ...(await menu.offer({ command: focus }, objects, palette.id)),
    ];
    await menu.set("settings", offered);
    await menu.choose("settings.focus");
    const focused = await objects.query.window.findFirst({ where: { id: view.id } });

    // expect it offered on the view only, which it focused now, and no item it no longer shows
    expect({
        sent,
        focuses: focused?.focuses,
        missing: await refusal(() => menu.choose("settings.close")),
    }).toEqual({
        sent: [
            {
                action: "menu.set",
                menu: "tray",
                items: [{ kind: "action", id: "settings.focus", title: "Focus" }],
            },
            {
                action: "menu.set",
                menu: "application",
                items: [{ kind: "action", id: "settings.focus", title: "Focus" }],
            },
        ],
        focuses: 1,
        missing: ["NOT_FOUND", "MENU_ITEM_NOT_FOUND", "no menu item settings.close runs"],
    });
});

test("run a chosen item by its identity after its section renders again, refusing one it no longer shows", async () => {
    // show checking for updates, then replace it with restarting before the choice arrives
    const { shell } = recordShell();
    const menu = new Menu(shell, Localization.of(SOURCE_LOCALE, []));
    const ran: string[] = [];
    const entry = (id: string): MenuEntry => ({
        kind: "action",
        id,
        title: id,
        run: async () => {
            ran.push(id);
        },
    });
    await menu.set("update", [entry("check")]);
    await menu.set("update", [entry("restart")]);

    // choose the item shown first, then the one shown now
    const missing = await refusal(() => menu.choose("update.check"));
    await menu.choose("update.restart");
    expect({ missing, ran }).toEqual({
        missing: ["NOT_FOUND", "MENU_ITEM_NOT_FOUND", "no menu item update.check runs"],
        ran: ["restart"],
    });
});
