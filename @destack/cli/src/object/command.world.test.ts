import { join } from "node:path";
import { card } from "@destack/machine-board-fixture/object";
import { schema } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { jsonLines, TerminalFixture } from "../test/terminal.ts";

/** A printed object's identifier. */
const Printed = schema.looseObject({ id: schema.string() });

/** Open the owner's home with the board published and installed under some names. */
async function open(...names: string[]): Promise<TerminalFixture> {
    // publish the board, then install it under each name
    const terminal = await TerminalFixture.open({ NO_COLOR: "1" });
    await terminal.publish("board");
    for (const name of names) {
        await terminal.install("board", name);
    }

    return terminal;
}

/** Run a command line in the home, returning what it printed. */
function line(terminal: TerminalFixture, words: readonly string[], isTerminal = false) {
    return terminal.printed(["--space", "home", ...words], { isTerminal });
}

/** Create a card of a title through the command line, returning its identifier. */
async function create(terminal: TerminalFixture, title: string): Promise<string> {
    const created = await line(terminal, ["card", "create", "--title", title, "--json"]);

    return Printed.parse(JSON.parse(created.output)).id;
}

/** Write the JSON line a failure prints on standard error. */
function failure(code: string, message: string): string {
    return `${JSON.stringify({ error: { code, message } })}\n`;
}

test("create, get, change and call a card by its singular noun, plain and qualified", async () => {
    // create, retitle, list the relationships of, and get a card
    const terminal = await open("work");
    const id = await create(terminal, "Plans");
    const created = await terminal.object(card, "work", id);
    const updated = await line(terminal, [
        "card",
        id,
        "update",
        "--title",
        "Ideas",
        "--revision",
        "1",
        "--json",
    ]);
    const related = await line(terminal, ["card", id, "relationships", "--json"]);
    const got = await line(terminal, ["work:card", id, "--json"]);

    // print each stored object as JSON
    const stored = await terminal.object(card, "work", id);
    expect({ created, updated, related, got }).toEqual({
        created: { ...stored, title: "Plans", revision: 1 },
        updated: { code: 0, output: `${JSON.stringify(stored)}\n`, error: "" },
        related: { code: 0, output: `${JSON.stringify({ items: [], cursor: null })}\n`, error: "" },
        got: { code: 0, output: `${JSON.stringify(stored)}\n`, error: "" },
    });
});

test("read a method's input from standard input and from a file", async () => {
    // write the input file
    const terminal = await open("work");
    const { files } = testHost();
    const directory = await files.makeTemporaryDirectory("destack-cli-input-");
    onTestFinished(() => files.remove(directory, { recursive: true }));
    const path = join(directory, "input.json");
    await files.writeTextFile(path, JSON.stringify({ title: "Ideas" }));
    const createFrom = ["--space", "home", "card", "create", "--json", "--input"];

    // create one card from standard input and one from the file
    const printed = [
        await terminal.printed([...createFrom, "-"], { stdin: JSON.stringify({ title: "Plans" }) }),
        await terminal.printed([...createFrom, `@${path}`]),
    ];
    const ids = printed.map(({ output }) => Printed.parse(JSON.parse(output)).id);
    const stored = await Promise.all(ids.map((id) => terminal.object(card, "work", id)));

    // print each card stored with its input's title
    expect({
        printed,
        titles: stored.map(
            (row) => schema.looseObject({ title: schema.string() }).parse(row).title,
        ),
    }).toEqual({
        printed: stored.map((row) => ({ code: 0, output: `${JSON.stringify(row)}\n`, error: "" })),
        titles: ["Plans", "Ideas"],
    });
});

test("list a type's objects by its plural, restricted by field flags, a filter, an order and a limit", async () => {
    // create three cards
    const terminal = await open("work");
    const [plans, ideas] = [await create(terminal, "Plans"), await create(terminal, "Ideas")];
    await create(terminal, "Drafts");

    // list the cards as a person's table, a program's tab-separated rows, and JSON
    const table = await line(
        terminal,
        [
            "cards",
            "--title",
            "Ideas,Plans",
            "--where",
            "title != Drafts",
            "--sort=-title",
            "--limit",
            "5",
            "--fields",
            "id,title",
        ],
        true,
    );
    const rows = await line(terminal, [
        "cards",
        "--fields",
        "id,title",
        "--where",
        "title != Drafts",
        "--sort=title",
    ]);
    const json = await line(terminal, ["work:cards", "--where", "title = Plans", "--json"]);

    // expect the matching cards in order, each as stored
    expect([table.output, rows.output, json.output]).toEqual([
        [
            "ID                                         TITLE",
            `${plans}  Plans`,
            `${ideas}  Ideas`,
            "",
        ].join("\n"),
        `${ideas}\tIdeas\n${plans}\tPlans\n`,
        `${JSON.stringify([await terminal.object(card, "work", plans)])}\n`,
    ]);
});

test("offer every object type an installation lists as a noun, and refuse a word two installations serve", async () => {
    // install the board, and as the archive another package declaring cards
    const terminal = await open("work");
    await terminal.publish("mention");
    await terminal.install("mention", "archive");
    const refused = async (words: string[]) => {
        const { code, error } = await line(terminal, words);

        return [code, error.trim()];
    };

    // refuse the bare nouns, and list a qualified one
    expect([
        await refused(["cards"]),
        await refused(["card", "card-01996ab0-0000-7000-8000-0000000000d1"]),
        await refused(["archive:cards", "--json"]),
    ]).toEqual([
        [2, "destack: cards is ambiguous: use one of archive:cards, work:cards"],
        [2, "destack: card is ambiguous: use one of archive:card, work:card"],
        [0, ""],
    ]);
});

test("refuse a list's limit outside the positive whole numbers, and an order starting with a dash without an equals sign", async () => {
    // list with a zero limit, and with a descending order as its own word
    const terminal = await open("work");
    const refused = async (words: string[]) => {
        const { code, error } = await line(terminal, words);

        return [code, error.trim()];
    };
    expect([
        await refused(["cards", "--limit", "0"]),
        await refused(["cards", "--sort", "-title"]),
    ]).toEqual([
        [2, "destack: --limit takes a positive whole number"],
        [
            2,
            "destack: --sort takes a value: write --sort=<value> when the value starts with a dash",
        ],
    ]);
});

test("report a refused call with its service code, exiting with 1", async () => {
    // update a card at a revision it moved past
    const terminal = await open("work");
    const id = await create(terminal, "Plans");
    await line(terminal, ["card", id, "update", "--title", "Ideas", "--revision", "1"]);
    expect(
        await line(terminal, [
            "card",
            id,
            "update",
            "--title",
            "Drafts",
            "--revision",
            "1",
            "--json",
        ]),
    ).toEqual({
        code: 1,
        output: "",
        error: failure("CONFLICT", "card revision has changed"),
    });
});

test("find an installation's nouns on the command line after its install", async () => {
    // list the work cards, then install the archive and list its cards by qualified noun
    const terminal = await open("work");
    const listed = async (words: string[]) => (await line(terminal, [...words, "--json"])).code;
    const before = [await listed(["cards"]), await listed(["archive:cards"])];
    await terminal.publish("mention");
    await terminal.install("mention", "archive");
    const after = await listed(["archive:cards"]);

    // expect the archive's noun unknown before its install and listed after it
    expect({ before, after }).toEqual({ before: [0, 2], after: 0 });
});

test("print help for a command line without words as destack help does, and a noun's methods and flags", async () => {
    // print the root's help, and the help of the cards' commands
    const terminal = await open("work");
    const root = await line(terminal, []);
    expect({
        root,
        card: (await line(terminal, ["card", "--help"])).output,
        create: (await line(terminal, ["card", "create", "--help"])).output,
    }).toEqual({
        root: await line(terminal, ["help"]),
        card: `Get, create and change cards (work)

Usage: destack card <command>

  create              Call card.create
  new-ideas           New Ideas Card
  <id>                Show one of the cards
  <id> pin            Call card.pin
  <id> update         Call card.update
  <id> relationships  Call card.relationships
  <id> grant          Call card.grant
  <id> revoke         Call card.revoke
  <id> invitations    Call card.invitations
  <id> invite         Call card.invite
  <id> accept         Call card.accept
  <id> explain        Call card.explain
  <id> withdraw       Call card.withdraw
  <id> detach         Call card.detach
  <id> attach         Call card.attach
  <id> retitle-ideas  Retitle to Ideas

  -h, --help              Show help
`,
        create: `Call card.create

Usage: destack card create

  -h, --help              Show help
      --json              Print JSON
      --title <string>    The title
      --input <json>      The method's input as a JSON object, - for stdin, @path for a file
`,
    });
});

test("pass a flag of identifiers to a method as given, refusing a key for a type without one", async () => {
    // create a space on the computer's machine by its identifier, then by a name machines do not key
    const terminal = await TerminalFixture.open();
    const machine = terminal.computer.machine.id;
    const createOn = (on: string) =>
        terminal.printed(["space", "create", "--name", "notes", "--machine", on, "--json"]);
    const created = await createOn(machine);
    const refused = await createOn("laptop");

    // expect the identifier passed through, and the name refused by the method's input
    const { id } = Printed.parse(JSON.parse(created.output));
    expect({ created, refused }).toEqual({
        created: {
            code: 0,
            output: (await terminal.printed(["space", id, "--json"])).output,
            error: "",
        },
        refused: {
            code: 1,
            output: "",
            error: failure(
                "BAD_REQUEST",
                "invalid input: machine: invalid string: must match pattern /^machine-[0-9a-f]{8}-[0-9a-f]{4}-[78][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u",
            ),
        },
    });
});

test("build a structured input from one flag per path, a union's variants each adding theirs", async () => {
    // read the selection's flags of an installation's create
    const terminal = await open();
    const help = await line(terminal, ["installation", "create", "--help"]);

    // offer each variant's fields of the selection
    expect(help).toEqual({
        code: 0,
        output: `Call installation.create

Usage: destack installation create

  -h, --help                                              Show help
      --json                                              Print JSON
      --package-id <id>                                   The package id
      --name <string>                                     The name
      --selection-kind <release|tag|repository|checkout>  The selection kind: release, tag, repository, checkout
      --selection-version <string>                        The selection version
      --selection-manifest <string>                       The selection manifest
      --selection-tag <string>                            The selection tag
      --selection-repository <id>                         The selection repository
      --selection-reference <string>                      The selection reference
      --selection-directory <string>                      The selection directory
      --selection-machine <id>                            The selection machine
      --selection-checkout <id>                           The selection checkout
      --export <string>                                   The export
      --status <enabled|suspended>                        The status: enabled, suspended
      --isolation <sandbox|process>                       The isolation: sandbox, process
      --input <json>                                      The method's input as a JSON object, - for stdin, @path for a file
`,
        error: "",
    });
});

test("run a declared command on an object as one of its verbs, setting its input, and refuse an object missing its condition", async () => {
    // retitle a card to Ideas, then try again once it is titled Ideas
    const terminal = await open("work");
    const id = await create(terminal, "Plans");
    const retitled = await line(terminal, [
        "card",
        id,
        "retitle-ideas",
        "--revision",
        "1",
        "--json",
    ]);
    const again = await line(terminal, ["card", id, "retitle-ideas", "--revision", "2", "--json"]);

    // expect the card retitled once, and the second refused
    const stored = await terminal.object(card, "work", id);
    expect({ retitled, again }).toEqual({
        retitled: { code: 0, output: `${JSON.stringify(stored)}\n`, error: "" },
        again: {
            code: 1,
            output: "",
            error: failure("PRECONDITION_FAILED", `retitle-ideas does not apply to card ${id}`),
        },
    });
});

test("run a declared command without a target as a verb of its noun, setting its input", async () => {
    // create a card through the noun's verb
    const terminal = await open("work");
    const created = await line(terminal, ["card", "new-ideas", "--json"]);
    const { id } = Printed.parse(JSON.parse(created.output));

    // expect the card created with the command's title
    expect(created).toEqual({
        code: 0,
        output: `${JSON.stringify(await terminal.object(card, "work", id))}\n`,
        error: "",
    });
});

test("watch a noun's objects as JSON lines, printing them again at each change, until interrupted", async () => {
    // watch the cards, then create one
    const terminal = await open("work");
    const watching = terminal.start(["--space", "home", "cards", "--watch", "--json"]);
    await watching.waitLines(1);
    const id = await create(terminal, "Plans");
    await watching.waitLines(2);

    // stop watching as Ctrl-C does
    watching.interrupt();
    const { code, output, error } = await watching.printed;
    const stored = await terminal.object(card, "work", id);

    // print no cards, then the created one, each as a JSON line, and exit as interrupted
    expect({ code, lines: jsonLines(output), error }).toEqual({
        code: 130,
        lines: [[], [stored]],
        error: "",
    });
});

test("watch one object as JSON lines, printing it again at each change, until interrupted", async () => {
    // watch a card, then retitle it
    const terminal = await open("work");
    const id = await create(terminal, "Plans");
    const watching = terminal.start(["--space", "home", "card", id, "--watch", "--json"]);
    await watching.waitLines(1);
    const created = await terminal.object(card, "work", id);
    await line(terminal, ["card", id, "update", "--title", "Ideas", "--revision", "1"]);
    await watching.waitLines(2);

    // stop watching as Ctrl-C does
    watching.interrupt();
    const { code, output, error } = await watching.printed;
    const updated = await terminal.object(card, "work", id);

    // print the card as created, then as retitled, and exit as interrupted
    expect({
        code,
        lines: jsonLines(output),
        error,
        titles: [created["title"], updated["title"]],
    }).toEqual({
        code: 130,
        lines: [created, updated],
        error: "",
        titles: ["Plans", "Ideas"],
    });
});
