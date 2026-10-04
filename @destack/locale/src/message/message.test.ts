import { PackageError } from "@destack/package";
import { expect, test } from "@destack/test";
import { Message, plural, select, t, verbatim } from "./message.ts";

/** The id of this package, whose modules the transform stamps. */
const LOCALE = "package-01a10754-6288-731f-a2ef-8b79479a427a";

test("write a placeholder per interpolation and escape the characters MessageFormat 2 reserves", () => {
    const greeting = t`Hello ${"Ana"}, use {braces} and | bars`;
    expect([greeting.source, greeting.values]).toEqual([
        "Hello {$p0}, use \\{braces\\} and \\| bars",
        { p0: "Ana" },
    ]);
});

test("write a plural as a numeric selector, # standing for the count, the catch-all variant last", () => {
    const archived = t`Archived ${plural(2, { other: "# notes", one: "# note", zero: "no notes" })}`;
    expect([archived.source.split("\n"), archived.values]).toEqual([
        [
            ".input {$p0 :number}",
            ".match $p0",
            "one {{Archived {$p0} note}}",
            "zero {{Archived no notes}}",
            "* {{Archived {$p0} notes}}",
        ],
        { p0: 2 },
    ]);
});

test("write one variant per combination of several selectors", () => {
    const message = t`${select("woman", { woman: "She", other: "They" })} shared ${plural(1, { one: "# note", other: "# notes" })}`;
    expect(message.source.split("\n")).toEqual([
        ".input {$p0 :string}",
        ".input {$p1 :number}",
        ".match $p0 $p1",
        "woman one {{She shared {$p1} note}}",
        "woman * {{She shared {$p1} notes}}",
        "* one {{They shared {$p1} note}}",
        "* * {{They shared {$p1} notes}}",
    ]);
});

test("identify a message by its source and context alone, not its values", () => {
    expect([
        t`Archived ${"Trips"}`.id === t`Archived ${"Work"}`.id,
        t`Open`.id === Message.context("button")`Open`.id,
        Message.context("button")`Open`.id === Message.context("button")`Open`.id,
    ]).toEqual([true, false, true]);
});

test("stamp messages with the package of the module writing them", () => {
    expect([t`Open`.package, Message.context("button")`Open`.package]).toEqual([LOCALE, LOCALE]);
});

test("refuse to write a message without the module the transform passes", () => {
    // call the tag and the context directly, as an untransformed module would
    const strings = Object.assign(["Open"], { raw: ["Open"] });
    expect(() => t(strings)).toThrow(
        new PackageError(
            "INVALID_DEFINITION",
            "no module passed by the Destack module transform to: t",
        ),
    );
    expect(() => Message.context("button", undefined)).toThrow(
        new PackageError(
            "INVALID_DEFINITION",
            "no module passed by the Destack module transform to: Message.context",
        ),
    );
});

test("keep verbatim text a plain string with its values in place", () => {
    expect(verbatim`Destack ${"2026.10.1"} on ${"macOS"}`).toBe("Destack 2026.10.1 on macOS");
});

test("keep a message's values as JSON it travels as, refusing a value that is no JSON", () => {
    const stored = Message.parse(JSON.parse(JSON.stringify(t`Saved ${3} notes in ${"Trips"}`)));
    expect(stored.values).toEqual({ p0: 3, p1: "Trips" });
    expect(() => t`Saved at ${new Map()}`).toThrow(
        new TypeError("message value p0 is no JSON value"),
    );
});
