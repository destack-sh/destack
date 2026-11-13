import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { TerminalFixture } from "../test/terminal.ts";

/** Run a command line through this computer's native device, returning what it prints. */
async function output(terminal: TerminalFixture, words: readonly string[]): Promise<string> {
    const shown = await terminal.printed([...words, "--json"]);
    expect(shown.error).toBe("");

    return shown.output;
}

test("set, list and reset a device setting in this computer's local objects", async () => {
    const terminal = await TerminalFixture.open();
    const link = "destack-local://home--home--acme/home";

    // set the start link at the client, read it back, then reset it
    const Shown = schema.looseObject({ value: schema.json(), state: schema.string() });
    const set = Shown.parse(
        JSON.parse(await output(terminal, ["setting", "start", "set", link, "--scope", "device"])),
    );
    const listed = schema
        .array(schema.looseObject({ name: schema.string(), value: schema.json() }))
        .parse(JSON.parse(await output(terminal, ["settings", "--scope", "device"])));
    await output(terminal, ["setting", "start", "reset", "--scope", "device"]);
    const reset = Shown.parse(
        JSON.parse(await output(terminal, ["setting", "start", "--scope", "device"])),
    );

    // store the value in the client's local objects until reset to the default
    expect({
        set: [set.value, set.state],
        listed: listed.map((row) => [row.name, row.value]),
        reset: [reset.value, reset.state],
    }).toEqual({
        set: [link, "set"],
        listed: [["start", link]],
        reset: [null, "default"],
    });
});

test("show a setting's effective value and where it comes from without --scope, bare for a program", async () => {
    const terminal = await TerminalFixture.open();
    const link = "destack-local://home--home--acme/home";
    await output(terminal, ["setting", "start", "set", link, "--scope", "device"]);

    // read the effective value as JSON, on a terminal and through a pipe
    const Resolution = schema.looseObject({
        value: schema.json(),
        sources: schema.array(schema.looseObject({ kind: schema.string() })),
        overridden: schema.array(schema.looseObject({ kind: schema.string() })),
    });
    const resolved = Resolution.parse(JSON.parse(await output(terminal, ["setting", "start"])));
    const shown = await terminal.printed(["setting", "start"], { isTerminal: true });
    const piped = await terminal.printed(["setting", "start"]);

    // take the device's value over the default, naming both on a terminal
    expect({
        resolved: [
            resolved.value,
            resolved.sources.map((source) => source.kind),
            resolved.overridden.map((source) => source.kind),
        ],
        shown: shown.output,
        piped: piped.output,
    }).toEqual({
        resolved: [link, ["device"], ["default"]],
        shown: `${link}\n  from  device\n  over  default\n`,
        piped: `${link}\n`,
    });
});

test("complete a boolean setting's values, and none for a setting taking any text", async () => {
    // complete the value of the autostart toggle and of the start link
    const terminal = await TerminalFixture.open();
    const toggle = await terminal.printed(["__complete", "setting", "autostart", "set", ""]);
    const link = await terminal.printed(["__complete", "setting", "start", "set", ""]);

    // expect both truth values for the toggle before its flags, and only flags for the link
    const flags = [
        "--scope\tWhere the value is placed: account, space or device",
        "--help\tShow help",
        "--json\tPrint JSON",
    ];
    expect([toggle.output, link.output]).toEqual([
        ["true\tvalue", "false\tvalue", ...flags, ""].join("\n"),
        [...flags, ""].join("\n"),
    ]);
});
