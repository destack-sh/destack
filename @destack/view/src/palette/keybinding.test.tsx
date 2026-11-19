import { PackageId } from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import { expect, test } from "@destack/test";
import { Keybinding } from "./keybinding.ts";

/** The package declaring the bound commands. */
const TASKS = PackageId.parse("package-019f7480-0000-7000-8000-000000000001");

test("bind a command to the person's keys, to none once unbound, else to its declared ones", () => {
    const complete = { packageId: TASKS, name: "complete" };
    const archive = { packageId: TASKS, name: "archive" };
    const pin = { packageId: TASKS, name: "pin" };
    const overrides = {
        [DeclarationReference.key(complete)]: "mod+shift+c",
        [DeclarationReference.key(archive)]: null,
    };

    expect([
        Keybinding.of(complete, { key: "mod+k", focus: "page" }, overrides),
        Keybinding.of(archive, { key: "mod+a", focus: "page" }, overrides),
        Keybinding.of(pin, { key: "mod+p", focus: "any" }, overrides),
        Keybinding.of({ ...pin, name: "star" }, { key: undefined, focus: "page" }, overrides),
    ]).toEqual([
        { command: complete, key: "mod+shift+c", focus: "page" },
        undefined,
        { command: pin, key: "mod+p", focus: "any" },
        undefined,
    ]);
});
