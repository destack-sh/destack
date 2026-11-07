import { window } from "@destack/desktop/object";
import { card } from "@destack/machine-board-fixture/object";
import { DesktopLock } from "@destack/desktop/workload";
import { schema } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { TerminalFixture } from "../test/terminal.ts";

/** Run a command line through this computer's native device, returning what it prints. */
async function output(terminal: TerminalFixture, words: readonly string[]): Promise<string> {
    const shown = await terminal.printed([...words, "--json"]);
    expect(shown.error).toBe("");

    return shown.output;
}

test("open a link as a window of the running desktop", async () => {
    const terminal = await TerminalFixture.open({ DESTACK_STAGE: "local" });
    const { native } = terminal;
    const link = "destack-local://home--home--acme/home/inbox";

    // open the link while a desktop runs
    await using desktop = await new DesktopLock(
        native.home,
        terminal.computer.environment,
    ).acquire();
    const answered = schema
        .object({ link: schema.string() })
        .parse(JSON.parse(await output(terminal, ["open", link])));

    // store it as a window row, which the desktop shows
    const windows = await native.objects({ window });
    onTestFinished(() => windows.close());
    const rows = await windows.query.window.findMany();

    // expect one view window of the link, which no desktop observed yet
    expect({ answered, windows: rows.map((row) => [row.link, row.kind, row.conditions]) }).toEqual({
        answered: { link },
        windows: [[link, "view", {}]],
    });
});

test("open an object by its identifier as a window of the running desktop", async () => {
    // create a card in the board installed in the owner's home
    const terminal = await TerminalFixture.open({ DESTACK_STAGE: "local" });
    await terminal.publish("board");
    await terminal.install("board", "work");
    const created = await terminal.printed([
        "--space",
        "home",
        "card",
        "create",
        "--title",
        "Plans",
        "--json",
    ]);
    const { id } = schema.object({ id: schema.string() }).loose().parse(JSON.parse(created.output));

    // open the card while a desktop runs
    await using desktop = await new DesktopLock(
        terminal.native.home,
        terminal.computer.environment,
    ).acquire();
    const answered = await output(terminal, ["--space", "home", "open", id]);

    // read the windows the desktop shows, and the card's link: its space's address opening the object
    const windows = await terminal.native.objects({ window });
    onTestFinished(() => windows.close());
    const rows = await windows.query.window.findMany();
    const { home } = terminal.computer.world.owner;
    const object = { packageId: card.policy.definition.packageId, type: "card", scope: home, id };
    const link = `destack-local://${terminal.address("home")}/.destack/open?object=${encodeURIComponent(JSON.stringify(object))}`;

    // store one view window of the card's link, which no desktop observed yet
    expect({
        answered: schema.json().parse(JSON.parse(answered)),
        windows: rows.map((row) => [row.link, row.kind]),
    }).toEqual({
        answered: { link },
        windows: [[link, "view"]],
    });
});

test("open a view in the browser through the person's opener, without asking when they installed it", async () => {
    // install the start page in a work space of the owner's, reaching their home's settings
    const terminal = await TerminalFixture.open({ DESTACK_STAGE: "local" });
    const work = await terminal.space("work");
    await terminal.install("start", "start", work);
    const link = `destack-local://start--${terminal.address("work")}/start`;

    // open it in the browser, which the installer granted what it asks
    const printed = await terminal.printed(["open", link, "--browser", "--json"]);
    const [url = ""] = terminal.opened();
    const launched = new URL(url);

    // open the launch once, on the installation's origin, printing and announcing it
    expect({
        printed,
        opened: terminal.opened().length,
        launch: [launched.hostname, launched.pathname, launched.searchParams.has("token")],
    }).toEqual({
        printed: { code: 0, output: `${JSON.stringify({ url })}\n`, error: `Opening ${url}\n` },
        opened: 1,
        launch: [`start--${terminal.address("work")}.localhost`, "/start", true],
    });
});
