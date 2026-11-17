import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { TerminalFixture } from "../test/terminal.ts";

/** The checkout Acme publishes its template from. */
const STARTER = fileURLToPath(new URL("../../tests/fixture/starter/", import.meta.url));

/** The identifier of Acme's template, which a package made from it replaces. */
const STARTER_ID = "package-01996ab0-0000-7000-8000-0000000000d5";

/** What `create --json` prints. */
const Created = schema.object({
    name: schema.string(),
    directory: schema.string(),
    checkout: schema.identifier("checkout"),
});

test("make a package from a published template in a new checkout, its registries at the universe", async () => {
    // create the acme organization as the owner, and publish its template
    const terminal = await TerminalFixture.open({ NO_COLOR: "1" });
    const words = ["organization", "create", "--name", "Acme", "--handle", "acme"];
    await terminal.printed([...words, "--residency-id", "eu"]);
    const published = await terminal.printed(["--space", "home", "publish", STARTER, "--json"], {
        stdin: "yes\n",
    });
    expect([published.code, published.error]).toEqual([0, ""]);

    // make a package from the template in a new directory
    const directory = join(terminal.native.home, "notes");
    const options = ["--template", "@acme/starter", "--json"];
    const printed = await terminal.printed(["create", "@acme/notes", directory, ...options]);
    expect([printed.code, printed.error]).toEqual([0, ""]);
    const created = Created.parse(JSON.parse(printed.output));

    // read what it wrote, its fresh identifier aside
    const files = terminal.native.environment.files;
    const read = async (path: string) => files.readTextFile(join(created.directory, path));
    const manifest: unknown = JSON.parse(await read("package.json"));
    const { id, ...definition } = schema
        .looseObject({ id: PackageId })
        .parse(JSON.parse(await read("destack.json")));
    const entries = await files.readDirectory(created.directory);
    const { universe } = await terminal.computer.machine.enrolled("making a package");
    expect({
        code: [printed.code, printed.error],
        name: created.name,
        entries: entries.map((entry) => entry.name).toSorted(),
        manifest,
        definition,
        isOwn: id !== STARTER_ID,
        bunfig: await read("bunfig.toml"),
    }).toEqual({
        code: [0, ""],
        name: "@acme/notes",
        entries: [".git", "bunfig.toml", "destack.json", "package.json", "src"],
        manifest: {
            name: "@acme/notes",
            type: "module",
            exports: { "./package": "./src/package.ts" },
            dependencies: { "@destack/locale": "latest", "@destack/object": "latest" },
        },
        definition: {
            $schema: "https://destack.app/schemas/2026.9.0/destack.json",
            role: "app",
            language: "typescript",
            runtimes: ["bun"],
        },
        isOwn: true,
        bunfig: [
            'preload = ["@destack/package/bun/preload"]',
            "",
            "[install.scopes]",
            `"@acme" = { url = "${universe}/npm/" }`,
            `"@destack" = { url = "${universe}/npm/" }`,
            "",
        ].join("\n"),
    });
});
