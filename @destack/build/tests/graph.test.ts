import { test } from "@destack/test";
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { Fixture } from "./fixture.ts";
import * as library from "./fixture/library/request.ts";

test.concurrent("reuse the graph file of each module an edit leaves unchanged", async ({
    expect,
}) => {
    // build the library, then reorder a function body in one module and build again
    await using input = await Fixture.open("library");
    await using before = await input.build(library.request);
    const note = join(input.source, "src/note.ts");
    const source = await readFile(note, "utf8");
    await writeFile(
        note,
        source.replace("return { title, complete: false };", "return { complete: false, title };"),
    );
    await using after = await input.build(library.request);

    // keep the untouched module's file byte for byte, and change the edited module's file and the root
    const [first, second] = await Promise.all([before.reader.graph(), after.reader.graph()]);
    const index = await readFile(
        join(after.directory, `graph/${second.modules["src/index.ts"]}.json`),
    );
    expect([
        second.modules["src/index.ts"] === first.modules["src/index.ts"],
        second.modules["src/note.ts"] === first.modules["src/note.ts"],
        after.manifest.lists.graph.digest === before.manifest.lists.graph.digest,
        index.equals(
            await readFile(join(before.directory, `graph/${first.modules["src/index.ts"]}.json`)),
        ),
    ]).toEqual([true, false, false, true]);
});
