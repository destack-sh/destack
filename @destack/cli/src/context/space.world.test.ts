import { currentSpace } from "@destack/account/native";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { TerminalFixture } from "../test/terminal.ts";

/** A printed space's identifier. */
const Created = schema.looseObject({ id: schema.identifier("space") });

/** The printed installations: each one's name, scope and package. */
const Installed = schema.array(
    schema.looseObject({
        name: schema.string(),
        scope: schema.string(),
    }),
);

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
    expect({ lines, setting: await terminal.native.setting(currentSpace) }).toEqual({
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

test("refuse a command line acting as nobody signed in, telling the person to sign in, also for a universe noun", async () => {
    // sign the owner out
    const terminal = await TerminalFixture.open();
    const { native } = terminal;
    await native.signOut(await native.signedIn(terminal.login.id));

    // ask who is signed in, then list spaces, which only a signed-in person's catalog offers
    expect({
        whoami: await terminal.printed(["whoami", "--json"]),
        spaces: await terminal.printed(["spaces"]),
    }).toEqual({
        whoami: {
            code: 4,
            output: "",
            error: `${JSON.stringify({ error: { code: "UNAUTHORIZED", message: "sign in with destack login" } })}\n`,
        },
        spaces: {
            code: 2,
            output: "",
            error: "destack: unknown command destack spaces: sign in with destack login for the universe's and spaces' commands\n",
        },
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

test("create a space on this computer's machine, switch to it, and list the installations its stack applied", async () => {
    // create the space on this computer's machine, and wait until it serves its stack
    const terminal = await TerminalFixture.open();
    const machine = terminal.computer.machine.id;
    const words = ["space", "create", "--name", "projects", "--machine", machine, "--json"];
    const created = await terminal.printed(words);
    const space = Created.parse(JSON.parse(created.output)).id;
    await terminal.wait(terminal.computer.ready(space));

    // switch to it, then list its installations
    const switched = await terminal.printed(["switch", "projects"]);
    const installations = await terminal.printed(["installations", "--json"]);
    const own = Installed.parse(JSON.parse(installations.output)).filter(
        (row) => row.scope === space,
    );

    // list the installations the space's stack declares
    expect({
        created: [created.code, created.error],
        switched,
        installations: own.map((row) => row.name).toSorted(),
    }).toEqual({
        created: [0, ""],
        switched: {
            code: 0,
            output: `Switched to ${terminal.address("projects")}.\n`,
            error: "",
        },
        installations: ["inbox", "message", "observability", "social", "stack", "vault"],
    });
});
