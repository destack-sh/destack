import { currentSpace } from "@destack/account/native";
import accountPackage from "@destack/account/package";
import objectPackage from "@destack/object/package";
import { schema } from "@destack/schema";
import settingPackage from "@destack/setting/package";
import universePackage from "@destack/universe/package";
import { expect, test } from "@destack/test";
import { TerminalFixture } from "../test/terminal.ts";

/** Write what whoami prints as JSON for the owner acting in a space. */
function whoami(terminal: TerminalFixture, space: string | null): string {
    return JSON.stringify({ ...terminal.login, space });
}

test("switch to a space as this computer's space setting, which --space overrides and whoami shows", async () => {
    // open the owner's home with a desk beside it
    const terminal = await TerminalFixture.open();
    const desk = await terminal.space("desk");
    const [deskAddress, homeAddress] = [terminal.address("desk"), terminal.address("home")];
    const missing = terminal.address("missing");
    const line = async (words: string[]) => {
        const { code, output, error } = await terminal.printed(words);

        return { code, output: output.trim(), error: error.trim() };
    };

    // list the universe's installations outside a space, by scope and package
    const outside = schema
        .array(schema.looseObject({ scope: schema.string(), packageId: schema.string() }))
        .parse(JSON.parse((await terminal.printed(["installations", "--json"])).output))
        .map((row) => [row.scope, row.packageId]);

    // act in no space, switch to the desk, then give another space for one command line
    const lines = [
        await line(["whoami", "--json"]),
        await line(["switch", deskAddress]),
        await line(["whoami"]),
        await line(["--space", homeAddress, "whoami", "--json"]),
        await line([`--space=${deskAddress}`, "installations", "--json"]),
        await line(["switch", missing, "--json"]),
        await line(["--space", "notes", "whoami"]),
        await line(["--space", "desk", "whoami", "--json"]),
        await line(["--space", "home", "whoami", "--json"]),
        await line(["--space"]),
    ];

    // list the desk's installations as its own store lists them
    const listed = await terminal.printed([`--space=${deskAddress}`, "installations", "--json"]);
    const installations = listed.output.trim();

    // set the space switched to as this computer's setting, resolving names and refusing a space nobody can open
    const { login } = terminal;
    expect({ outside, lines, setting: await terminal.native.setting(currentSpace) }).toEqual({
        outside: [
            ["universe", accountPackage.id],
            ["universe", objectPackage.id],
            ["universe", settingPackage.id],
            ["universe", universePackage.id],
        ],
        lines: [
            { code: 0, output: whoami(terminal, null), error: "" },
            { code: 0, output: `Switched to ${deskAddress}.`, error: "" },
            {
                code: 0,
                output: [
                    `Name     ${login.name}`,
                    `User     ${login.subjectId}`,
                    `Issuer   ${login.issuer}`,
                    "Session  authenticated",
                    `Space    ${deskAddress}`,
                ].join("\n"),
                error: "",
            },
            { code: 0, output: whoami(terminal, homeAddress), error: "" },
            { code: 0, output: installations, error: "" },
            {
                code: 1,
                output: "",
                error: JSON.stringify({
                    error: { code: "NOT_FOUND", message: `nothing responds at ${missing}` },
                }),
            },
            { code: 1, output: "", error: "destack: no space of yours is called notes" },
            { code: 0, output: whoami(terminal, deskAddress), error: "" },
            { code: 0, output: whoami(terminal, homeAddress), error: "" },
            { code: 2, output: "", error: "destack: --space takes a value" },
        ],
        setting: desk,
    });
});

test("refuse a command line acting as nobody signed in, telling the person to sign in and exiting 4", async () => {
    // sign the owner out
    const terminal = await TerminalFixture.open();
    const { native } = terminal;
    await native.signOut((await native.session(terminal.login.id)).login);

    // ask who is signed in
    expect(await terminal.printed(["whoami", "--json"])).toEqual({
        code: 4,
        output: "",
        error: `${JSON.stringify({ error: { code: "UNAUTHORIZED", message: "sign in with destack login" } })}\n`,
    });
});

test("act in the space DESTACK_SPACE gives when the command line gives no --space", async () => {
    // act in the home by the variable
    const terminal = await TerminalFixture.open({ DESTACK_SPACE: "home" });
    const missing = terminal.address("missing");

    // ask who acts where without --space, then in a space --space gives instead
    const lines = [
        await terminal.printed(["whoami", "--json"]),
        await terminal.printed(["--space", missing, "whoami", "--json"]),
    ];

    // take the variable's space, which --space overrides
    expect(lines).toEqual([
        { code: 0, output: `${whoami(terminal, terminal.address("home"))}\n`, error: "" },
        {
            code: 1,
            output: "",
            error: `${JSON.stringify({ error: { code: "NOT_FOUND", message: `nothing responds at ${missing}` } })}\n`,
        },
    ]);
});
