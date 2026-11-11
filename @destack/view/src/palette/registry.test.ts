import { Localization, SOURCE_LOCALE, t } from "@destack/locale";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { CommandRegistry } from "./registry.ts";

test("register a view's commands by name, refuse a name taken, run one with its parsed input, and forget it once unregistered", async () => {
    // register a command without input and one asking for a line, recording what runs
    const ran: string[] = [];
    const registry = new CommandRegistry();
    const unregister = registry.register({
        name: "copy-link",
        title: t`Copy Link`,
        keybinding: "mod+shift+c",
        run: () => void ran.push("copy"),
    });
    registry.register({
        name: "go-to-line",
        title: t`Go to Line…`,
        input: schema.object({ line: schema.number().int().min(1) }),
        run: ({ line }) => void ran.push(`line ${line}`),
    });

    // try the name again, run both, refuse a line the schema refuses, then unregister the copy
    const taken = await Promise.try(() =>
        registry.register({ name: "copy-link", title: t`Copy`, run: () => {} }),
    ).then(
        () => "registered",
        (error: unknown) => String(error),
    );
    await registry.run("copy-link", {});
    await registry.run("go-to-line", { line: 12 });
    const invalid = await registry.run("go-to-line", { line: 0 }).then(
        () => "ran",
        (error: unknown) => (error instanceof Error ? error.name : "unknown"),
    );
    const listed = registry.list(Localization.of(SOURCE_LOCALE, [])).map((command) => command.name);
    unregister();

    // expect both listed, each run once with its input, the name and the line refused, the copy gone
    expect({
        listed,
        ran,
        taken,
        invalid,
        after: registry.list(Localization.of(SOURCE_LOCALE, [])).map((command) => command.name),
        missing: await registry.run("copy-link", {}).catch((error: unknown) => String(error)),
    }).toEqual({
        listed: ["copy-link", "go-to-line"],
        ran: ["copy", "line 12"],
        taken: "TypeError: the view already registers a command 'copy-link'",
        invalid: "ZodError",
        after: ["go-to-line"],
        missing: "TypeError: the view registers no command 'copy-link'",
    });
});

test("track one component's selection, refuse a second tracker, and read none once it stops", () => {
    // track a selected note, then try a second tracker
    const registry = new CommandRegistry();
    const note = {
        packageId: PackageId.parse("package-019f7480-0000-7000-8000-000000000001"),
        type: "note",
        scope: "space-019f7480-0000-7000-8000-000000000002",
        id: "note-019f7480-0000-7000-8000-000000000003",
    };
    const stop = registry.select(() => [note]);
    const second = (() => {
        try {
            registry.select(() => []);

            return "tracked";
        } catch (error) {
            return String(error);
        }
    })();
    const selected = registry.selection();
    stop();

    // expect the note read, the second tracker refused, and no selection after the first stops
    expect({ selected, second, after: registry.selection() }).toEqual({
        selected: [note],
        second: "TypeError: the view already tracks a selection",
        after: [],
    });
});
