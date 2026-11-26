import { PUBLISHER } from "@destack/machine/computer/test";
import { join } from "node:path";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { TerminalFixture } from "../test/terminal.ts";
import { testHost } from "@destack/test/host";

/** The identifier of Example's template, which a package made from it replaces. */
const STARTER_ID = "package-01996ab0-0000-7000-8000-0000000000d5";

/** What `new --json` prints. */
const Created = schema.object({
    name: schema.string(),
    directory: schema.string(),
    checkout: schema.identifier("checkout"),
});

/** Run git in a working tree, answering its trimmed output. */
async function git(directory: string, ...command: readonly string[]): Promise<string> {
    const ran = testHost().process.spawn(["git", "-C", directory, ...command]);
    const [output, code] = await Promise.all([new Response(ran.stdout).text(), ran.exited]);
    expect(code).toBe(0);

    return output.trim();
}

test("make a package from a published template in a new repository and its checkout, installed, committed and pushed to its forge", async () => {
    // open the owner's home, the template published by their example organization
    const published = { organizations: [PUBLISHER], published: ["starter"] };
    const terminal = await TerminalFixture.open({ NO_COLOR: "1" }, [], published);

    // make a package from the template in a new repository, checked out in a directory
    const directory = join(terminal.native.home, "notes");
    const options = ["--template", "@example/starter", "--json"];
    const printed = await terminal.printed(["new", "@example/notes", directory, ...options]);
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
        origin: /url = (?<url>\S+)/u.exec(await read(".git/config"))?.groups?.["url"],
        commit: await git(created.directory, "log", "-1", "--format=%s"),
        status: await git(created.directory, "status", "--porcelain"),
    }).toEqual({
        code: [0, ""],
        name: "@example/notes",
        entries: [
            ".git",
            "bun.lock",
            "bunfig.toml",
            "destack.json",
            "node_modules",
            "package.json",
            "src",
        ],
        manifest: {
            name: "@example/notes",
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
            `"@destack" = { url = "${universe.origin}/.npm/" }`,
            `"@example" = { url = "${universe.origin}/.npm/" }`,
            "",
        ].join("\n"),
        origin: `${universe.origin}/@example/notes.git`,
        commit: "Introduce @example/notes",
        status: "",
    });
});
