import { test } from "@destack/test";
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { graph } from "@destack/package";
import { schema } from "@destack/schema";
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

test.concurrent("declare each test in its module's graph covering the package symbols it calls", async ({
    expect,
}) => {
    // add a suite whose test calls the library's note constructor
    await using input = await Fixture.open("library");
    const manifest = join(input.source, "package.json");
    const declared = schema.looseObject({}).parse(JSON.parse(await readFile(manifest, "utf8")));
    await writeFile(
        manifest,
        JSON.stringify({ ...declared, devDependencies: { "@destack/test": "workspace:*" } }),
    );
    await writeFile(
        join(input.source, "src/note.test.ts"),
        `import { describe, test } from "@destack/test";
import { createNote } from "./note.ts";
describe("note", () => {
    test("starts incomplete", () => {
        createNote("title");
    });
});
`,
    );
    await using build = await input.build(library.request);

    // read the test module's declarations and the edges from its test
    const root = await build.reader.graph();
    const module = graph.Module.parse(
        JSON.parse(
            await readFile(
                join(build.directory, `graph/${root.modules["src/note.test.ts"]}.json`),
                "utf8",
            ),
        ),
    );
    const registered = module.declarations.find((declaration) => declaration.kind === "test");
    expect({
        declarations: module.declarations.map(({ kind, name }) => ({ kind, name })),
        covers: module.edges
            .filter((edge) => edge.from === registered?.moniker && edge.kind === "covers")
            .map((edge) => edge.to.slice(edge.to.indexOf("/") + 1)),
    }).toEqual({
        declarations: [
            { kind: "test", name: "note › starts incomplete" },
            { kind: "suite", name: "note" },
        ],
        covers: ["src/note.ts#createNote"],
    });
});
