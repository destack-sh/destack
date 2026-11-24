import { TerminalSize } from "@destack/environment";
import { join } from "node:path";
import { card } from "@example/board/object";
import { PART_BYTES } from "@destack/object";
import { schema } from "@destack/schema";
import { expect, onTestFinished, single, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { PUBLISHER, type TestInstall, WORK_BOARD } from "@destack/machine/computer/test";
import { jsonLines, TerminalFixture } from "../test/terminal.ts";

/** A bucket identifier for reading its verbs' help. */
const BUCKET = "bucket-01996ab0-0000-7000-8000-000000000001";

/** A printed object's identifier. */
const Printed = schema.looseObject({ id: schema.string() });

/** Open the owner's home, with fixtures its template installed. */
function open(installs: readonly TestInstall[] = []): Promise<TerminalFixture> {
    return TerminalFixture.open({ NO_COLOR: "1" }, installs);
}

/** Run a command line in the home, returning what it printed. */
function line(terminal: TerminalFixture, words: readonly string[], isTerminal = false) {
    return terminal.printed(
        ["--space", "home", ...words],
        isTerminal ? { terminal: TerminalSize.VT100 } : {},
    );
}

/** Create a card of a title through the command line, returning its identifier. */
async function create(terminal: TerminalFixture, title: string): Promise<string> {
    const created = await line(terminal, ["card", "create", "--title", title, "--json"]);

    return Printed.parse(JSON.parse(created.output)).id;
}

/** Write the JSON line a failure prints on standard error. */
function failure(code: string, message: string): string {
    return `${JSON.stringify({ error: { code, message, causes: [] } })}\n`;
}

test("create, get, change and call a card by its singular noun, plain and qualified", async () => {
    // create, retitle, list the relationships of, and get a card
    const terminal = await open(WORK_BOARD);
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

test("create and get a card through its plural noun", async () => {
    // create a card and get it through the plural
    const terminal = await open(WORK_BOARD);
    const created = await line(terminal, ["cards", "new", "--title", "Plans", "--json"]);
    const { id } = Printed.parse(JSON.parse(created.output));
    const got = await line(terminal, ["cards", id, "--json"]);

    // create it as the singular does, and print the stored card
    const stored = await terminal.object(card, "work", id);
    expect({ created, got }).toEqual({
        created: { code: 0, output: `${JSON.stringify(stored)}\n`, error: "" },
        got: { code: 0, output: `${JSON.stringify(stored)}\n`, error: "" },
    });
});

test("read a method's input from standard input and from a file", async () => {
    // write the input file
    const terminal = await open(WORK_BOARD);
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
    const terminal = await open(WORK_BOARD);
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
            "  id                                         title",
            `  ${"─".repeat(41)}  ${"─".repeat(5)}`,
            `  ${plans}  Plans`,
            `  ${ideas}  Ideas`,
            "",
            "  2 cards",
            "",
        ].join("\n"),
        `${ideas}\tIdeas\n${plans}\tPlans\n`,
        `${JSON.stringify([await terminal.object(card, "work", plans)])}\n`,
    ]);
});

test("offer every object type an installation lists as a noun, and refuse a word two installations serve", async () => {
    // open the work board with mention published by example
    const terminal = await TerminalFixture.open({ NO_COLOR: "1" }, WORK_BOARD, {
        organizations: [PUBLISHER],
        published: ["mention"],
    });

    // list the work cards and the archive's cards before the archive's install
    const listed = async (words: string[]) => {
        const { code, output, error } = await line(terminal, words);

        return [code, output.trim(), error.trim()];
    };
    const before = [await listed(["cards", "--json"]), await listed(["archive:cards", "--json"])];

    // install as the archive another package declaring cards
    await terminal.install("mention", "archive");

    // refuse the bare nouns, and list a qualified one
    const after = [
        await listed(["cards"]),
        await listed(["card", "card-01996ab0-0000-7000-8000-0000000000d1"]),
        await listed(["archive:cards", "--json"]),
    ];
    expect({ before, after }).toEqual({
        before: [
            [0, "[]", ""],
            [2, "", failure("BAD_REQUEST", "unknown command destack archive:cards").trim()],
        ],
        after: [
            [
                2,
                "",
                "error  noun 'cards' is invalid: it is ambiguous, so use one of 'archive:cards, work:cards'",
            ],
            [
                2,
                "",
                "error  noun 'card' is invalid: it is ambiguous, so use one of 'archive:card, work:card'",
            ],
            [0, "[]", ""],
        ],
    });
});

test("refuse a list's zero limit and a dash value without an equals sign", async () => {
    // list with a zero limit, and with a descending order as its own word
    const terminal = await open(WORK_BOARD);
    const refused = async (words: string[]) => {
        const { code, error } = await line(terminal, words);

        return [code, error.trim()];
    };
    expect([
        await refused(["cards", "--limit", "0"]),
        await refused(["cards", "--sort", "-title"]),
    ]).toEqual([
        [2, "error  --'limit' is invalid: it takes a positive whole number"],
        [
            2,
            "error  option '--sort' is invalid: it takes a value, so write '--sort'=<value> when the value starts with a dash",
        ],
    ]);
});

test("report a refused call with its service code, exiting with 1", async () => {
    // update a card at a revision it moved past
    const terminal = await open(WORK_BOARD);
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
        error: failure("CONFLICT", "cannot update card: its revision changed"),
    });
});

test("print help for a command line without words as destack help does, and a noun's methods and flags", async () => {
    // print the root's help, and the help of the cards' commands
    const terminal = await open(WORK_BOARD);
    const root = await line(terminal, []);
    expect({
        root,
        journeys: root.output.split("\n").slice(0, 12),
        card: (await line(terminal, ["card", "--help"])).output,
        create: (await line(terminal, ["card", "create", "--help"])).output,
        new: (await line(terminal, ["card", "new", "--help"])).output,
    }).toEqual({
        root: await line(terminal, ["help"]),
        // list the verbs by journey, a line each, then the stores' nouns wrapped under one word
        journeys: [
            "Destack dev: destack <nouns> to list, destack <noun> <id> [method] to get and call",
            "",
            "Usage: destack <command>",
            "",
            "  you       login  logout  whoami  self  completion",
            "  spaces    switch",
            "  apps      install  uninstall  open  apply  plan  approve  reject",
            "  share     share  invited  accept",
            "  build     new  dev  check  fix  test  build  publish  tag  git",
            "  settings  setting  settings",
            "  objects   computers  machine:visits  sign-ins  windows  distributions",
            "            checkouts  previews  users  account:sessions  user-logins  passkeys",
        ],
        card: `Get, create and change cards (work)

Usage: destack card <command>

  new                 Call card.create
  new-ideas           New Ideas Card
  <id>                Show one of the cards
  <id> download       Download a file
  <id> update         Call card.update
  <id> pin            Call card.pin
  <id> relationships  Call card.relationships
  <id> grant          Call card.grant
  <id> revoke         Call card.revoke
  <id> explain        Call card.explain
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
      --cover <path>      The cover, from a file, null for none
      --input <json>      The method's input as a JSON object, - for stdin, @path for a file
`,
        new: `Call card.create

Usage: destack card new

  -h, --help              Show help
      --json              Print JSON
      --title <string>    The title
      --cover <path>      The cover, from a file, null for none
      --input <json>      The method's input as a JSON object, - for stdin, @path for a file
`,
    });
});

test("pass a flag with an object's identifier or key, refusing an unknown key", async () => {
    // create spaces on the computer's cloud machine by its identifier and by its declared name
    const terminal = await open();
    const machine = terminal.computer.machine.id;
    const { machines } = terminal.computer.world.universe.universe.definition.platform;
    const { name } = single(machines.filter((declared) => declared.id === machine));
    const createOn = (space: string, on: string) =>
        terminal.printed(["space", "create", "--name", space, "--machine", on, "--json"]);
    const byIdentifier = await createOn("notes", machine);
    const byName = await createOn("tasks", name);
    const unknown = await createOn("chat", "laptop");

    // print each created space as a read of it does, and refuse the unknown key
    const read = async (created: { readonly output: string }) =>
        (await terminal.printed(["space", Printed.parse(JSON.parse(created.output)).id, "--json"]))
            .output;
    expect({ byIdentifier, byName, unknown }).toEqual({
        byIdentifier: { code: 0, output: await read(byIdentifier), error: "" },
        byName: { code: 0, output: await read(byName), error: "" },
        unknown: {
            code: 1,
            output: "",
            error: failure("NOT_FOUND", "'machine' with key 'laptop' not found"),
        },
    });
});

test("list a union input's flags in a method's help, one per path", async () => {
    // read the flags of a bucket's open
    const terminal = await open();
    const help = await line(terminal, ["bucket", BUCKET, "open", "--help"]);

    // offer the fields of both modes, the write's metadata one flag per path
    expect(help).toEqual({
        code: 0,
        output: `Call bucket.open

Usage: destack bucket ${BUCKET} open

  -h, --help                              Show help
      --json                              Print JSON
      --mode <read|write>                 The mode: read, write
      --key <string>                      The key
      --range <string>                    The range
      --if-match <string>                 The if match
      --customer-key <string>             The customer key
      --size <integer>                    The size
      --http-metadata-content-type <string>
                                          The http metadata content type
      --http-metadata-content-language <string>
                                          The http metadata content language
      --http-metadata-content-disposition <string>
                                          The http metadata content disposition
      --http-metadata-content-encoding <string>
                                          The http metadata content encoding
      --http-metadata-cache-control <string>
                                          The http metadata cache control
      --http-metadata-cache-expiry <integer>
                                          The http metadata cache expiry
      --if-none-match <*>                 The if none match: *
      --input <json>                      The method's input as a JSON object, - for stdin, @path for a file
`,
        error: "",
    });
});

test("run a declared action on an object, refusing one its condition excludes", async () => {
    // retitle a card to Ideas, then try again once it is titled Ideas
    const terminal = await open(WORK_BOARD);
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
            error: failure(
                "PRECONDITION_FAILED",
                `cannot run 'retitle-ideas' on card '${id}': it does not apply`,
            ),
        },
    });
});

test("run a declared action without a target as a verb of its noun, setting its input", async () => {
    // create a card through the noun's verb
    const terminal = await open(WORK_BOARD);
    const created = await line(terminal, ["card", "new-ideas", "--json"]);
    const { id } = Printed.parse(JSON.parse(created.output));

    // expect the card created with the command's title
    expect(created).toEqual({
        code: 0,
        output: `${JSON.stringify(await terminal.object(card, "work", id))}\n`,
        error: "",
    });
});

test("create a platform repository in the account's registry by its noun, and list it", async () => {
    // create a platform repository of the owner's account, whose registry its residency's home serves
    const terminal = await open();
    const { account } = terminal.computer.world.owner;
    const repository = ["--account", account, "--name", "site", "--hosting", "platform"];
    // list the registry's repositories after creating one
    const created = await line(terminal, ["repository", "new", ...repository, "--json"]);
    const listed = await line(terminal, ["repositories", "--json"]);
    const Repository = schema.looseObject({ name: schema.string(), hosting: schema.string() });

    // expect the repository created, and listed by its name and hosting
    expect({
        created: [created.code, created.error, Repository.parse(JSON.parse(created.output)).name],
        listed: schema
            .array(Repository)
            .parse(JSON.parse(listed.output))
            .map(({ name, hosting }) => [name, hosting]),
    }).toEqual({ created: [0, "", "site"], listed: [["site", "platform"]] });
});

test("watch a noun's objects as JSON lines, printing them again at each change, until interrupted", async () => {
    // watch the cards, then create one
    const terminal = await open(WORK_BOARD);
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
    const terminal = await open(WORK_BOARD);
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

test("send a file through a flag and download it, refusing a wrong file and to replace one", async () => {
    // write a picture longer than one part, and a text
    const terminal = await open(WORK_BOARD);
    const { files } = testHost();
    const directory = await files.makeTemporaryDirectory("destack-cli-file-");
    onTestFinished(() => files.remove(directory, { recursive: true }));
    const picture = join(directory, "Lisboa é.JPG");
    const bytes = new Uint8Array(PART_BYTES + 1000).map((_, place) => place % 251);
    await files.writeFile(picture, bytes);
    const text = join(directory, "notes.txt");
    await files.writeTextFile(text, "notes");

    // create a card with it, and download it over another file, its own name taken
    const send = (...words: readonly string[]) =>
        line(terminal, ["card", "create", "--title", "Lisbon", ...words, "--json"]);
    const made = schema
        .looseObject({ id: schema.string(), cover: schema.looseObject({}) })
        .parse(JSON.parse((await send("--cover", picture)).output));
    const copy = join(directory, "copy.jpg");
    await files.writeTextFile(copy, "old");
    const download = (...words: readonly string[]) =>
        line(terminal, ["card", made.id, "download", ...words, "--json"]);
    const saved = await download("--output", copy);
    const named = await download("cover", "--output", copy);
    const unknown = await download("photo");

    // clear it
    const cleared = await line(terminal, ["card", made.id, "update", "--cover", "null", "--json"]);
    const without = await download();
    const { signature, parts, ...cover } = made.cover;

    expect({
        cover,
        isSigned: typeof signature,
        parts: schema.array(schema.string()).parse(parts).length,
        saved: schema.json().parse(JSON.parse(saved.output)),
        named: named.code,
        isSame: (await files.readFile(copy)).toHex() === bytes.toHex(),
        unknown: unknown.error,
        cleared: schema.looseObject({ cover: schema.json() }).parse(JSON.parse(cleared.output))
            .cover,
        without: without.error,
        missing: (await send("--cover", "/none.png")).error,
        folder: (await send("--cover", directory)).error,
        wrong: (await send("--cover", text)).error,
    }).toEqual({
        cover: {
            name: "Lisboa é.JPG",
            picture: null,
            size: bytes.byteLength,
            type: "image/jpeg",
        },
        isSigned: "string",
        parts: 2,
        saved: { path: copy, size: bytes.byteLength, type: "image/jpeg" },
        named: 0,
        isSame: true,
        unknown: failure("BAD_REQUEST", "file 'photo' is invalid: 'card' has 'cover'"),
        cleared: null,
        without: failure("BAD_REQUEST", `card '${made.id}' is invalid: its 'cover' has no file`),
        missing: failure("BAD_REQUEST", "--'cover' file '/none.png' is invalid: it does not exist"),
        folder: failure(
            "BAD_REQUEST",
            `--'cover' file '${directory}' is invalid: it is not a file`,
        ),
        wrong: failure(
            "BAD_REQUEST",
            `--'cover' file '${text}' is invalid: it takes no file of type text/plain`,
        ),
    });
});
