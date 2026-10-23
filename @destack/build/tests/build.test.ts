import { refusal, test } from "@destack/test";
import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { PackageBuild } from "../src/index.ts";
import {
    Fixture,
    expectFiles,
    expectManifest,
    exported,
    importBuilt,
    isNoteModule,
} from "./fixture.ts";
import * as library from "./fixture/library/request.ts";
import { BuildReader } from "@destack/package/manifest";
import { Catalog } from "@destack/locale";
import { PackageDefinition } from "@destack/package";

test.concurrent("relocate the library build and call its exports, refusing corrupted description bytes", async ({
    expect,
}) => {
    await using input = await Fixture.open("library");
    await using build = await input.build(library.request);

    // load the complete distribution after removing its source checkout
    const destination = join(input.directory, "build");
    await PackageBuild.write(build.reader, destination);
    await rm(input.source, { recursive: true });
    const restored = await PackageBuild.read(destination);
    expect(restored.manifest).toEqual(build.manifest);
    await expectFiles(restored, build);
    await expectManifest(build, destination);

    // reject corrupted description bytes
    const corrupt = new BuildReader(build.manifest, async (path) => {
        const bytes = new Uint8Array(await readFile(join(destination, path)));
        const [first = 0] = bytes;
        bytes[0] = first ^ 1;

        return new Blob([bytes]).stream();
    });
    expect(await refusal(corrupt.files())).toEqual([
        "BAD_REQUEST",
        "INVALID_FILE",
        `file digest mismatch: ${build.manifest.lists.files.path}`,
    ]);

    // call the library's exports from the relocated build
    const module = await importBuilt(destination, exported(build.manifest.outputs["library"], "."));
    if (!isNoteModule(module)) {
        throw new TypeError("the library exports no createNote function");
    }
    expect(module.createNote("A note")).toEqual({ title: "A note", complete: false });
});

test.concurrent("ship a package's catalogs in its build, refusing a misnamed one and another package's", async ({
    expect,
}) => {
    // write a German catalog of the library's messages
    await using input = await Fixture.open("library");
    const owner = PackageDefinition.read(
        await readFile(join(input.source, "destack.json"), "utf8"),
    ).id;
    const german = { package: owner, locale: "de", messages: { a1: "Notiz" }, drafts: ["a1"] };
    const write = (name: string, catalog: object) =>
        writeFile(join(input.source, "locale", name), JSON.stringify(catalog));
    await mkdir(join(input.source, "locale"));
    await write("de.json", german);

    // read the shipped catalog back from the build
    await using build = await input.build(library.request);
    const shipped = await Catalog.read(build.reader);

    // refuse a catalog named for another locale, then one of another package
    await write("de.json", { ...german, locale: "de-AT" });
    const misnamed = input.build(library.request);
    expect(await refusal(misnamed)).toEqual([
        "UNPROCESSABLE_CONTENT",
        "CATALOG_INVALID",
        "invalid catalog: locale/de.json",
    ]);
    await write("de.json", { ...german, package: "package-01996ab0-0000-7000-8000-0000000000aa" });
    const foreign = input.build(library.request);
    expect(await refusal(foreign)).toEqual([
        "UNPROCESSABLE_CONTENT",
        "CATALOG_INVALID",
        "invalid catalog: locale/de.json",
    ]);
    expect(shipped).toEqual([german]);
});
