import { ActionKeybinding } from "@destack/object";
import { PackageId } from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import { expect, test } from "@destack/test";
import { Keybinding } from "./keybinding.ts";

/** The package declaring the bound actions. */
const TASKS = PackageId.parse("package-019f7480-0000-7000-8000-000000000001");

test("bind an action to the person's keys, else to its declared ones", () => {
    const complete = { packageId: TASKS, name: "complete" };
    const archive = { packageId: TASKS, name: "archive" };
    const pin = { packageId: TASKS, name: "pin" };
    const overrides = {
        [DeclarationReference.key(complete)]: "mod+shift+c",
        [DeclarationReference.key(archive)]: null,
    };

    const one = { keybinding: ActionKeybinding.of("mod+k"), focus: "page" } as const;
    const several = {
        keybinding: ActionKeybinding.of({ key: ["mod+p", "p"], mac: "mod+o" }),
        focus: "any",
    } as const;

    expect([
        Keybinding.of(complete, one, overrides, false),
        Keybinding.of(archive, one, overrides, false),
        Keybinding.of(pin, several, overrides, false),
        Keybinding.of(pin, several, overrides, true),
        Keybinding.of(
            { ...pin, name: "star" },
            { keybinding: undefined, focus: "page" },
            overrides,
            false,
        ),
    ]).toEqual([
        [{ action: complete, key: "mod+shift+c", focus: "page" }],
        [],
        [
            { action: pin, key: "mod+p", focus: "any" },
            { action: pin, key: "p", focus: "any" },
        ],
        [{ action: pin, key: "mod+o", focus: "any" }],
        [],
    ]);
});
