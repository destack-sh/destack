import { window } from "@destack/desktop/object";
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
