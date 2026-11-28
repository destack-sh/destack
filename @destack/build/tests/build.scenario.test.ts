import { testSimulation } from "../src/test/world.ts";
import { refusal, test } from "@destack/test";
import { join } from "node:path";
import { PackageBuild } from "../src/index.ts";
import { COMPILER_VERSION } from "../src/cache/fingerprint.ts";
import { Digest } from "@destack/schema";
import {
    Fixture,
    expectFiles,
    expectManifest,
    exported,
    importBuilt,
    isNoteModule,
} from "./fixture.ts";
import * as library from "./fixture/library/request.ts";
import { BuildReader, BuildListCache } from "@destack/package/manifest";
import { Catalog } from "@destack/locale";
import { PackageDefinition } from "@destack/package";

test.concurrent("relocate the library build and call its exports, refusing corrupted description bytes", async ({
    expect,
}) => {
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    await using input = await Fixture.open("library", environment);
    const build = await input.build(library.request);

    // load the complete distribution after removing its source checkout
    const destination = join(input.directory, "build");
    await build.writeDirectory(destination, files);
    await files.remove(input.source, { recursive: true });
    const restored = await PackageBuild.read(destination, environment);
    expect(restored.manifest).toEqual(build.manifest);
    await expectFiles(restored.reader, build);
    await expectManifest(build, destination, environment);

    // reject corrupted description bytes
    const corrupt = new BuildReader(
        build.manifest,
        async (path) => {
            const bytes = new Uint8Array(await files.readFile(join(destination, path)));
            const [first = 0] = bytes;
            bytes[0] = first ^ 1;

            return new Blob([bytes]).stream();
        },
        environment.crypto,
        new BuildListCache(),
    );
    expect(await refusal(corrupt.files())).toEqual([
        "BAD_REQUEST",
        "FILE_INVALID",
        `file '${build.manifest.lists.files.path}' is invalid: its digest does not match`,
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
    // write a German catalog source of the library's messages
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    await using input = await Fixture.open("library", environment);
    const owner = PackageDefinition.read(
        await files.readTextFile(join(input.source, "destack.json")),
    ).id;
    const german = {
        package: owner,
        locale: "de",
        messages: { a1: { source: "Note", translation: "Notiz" } },
    };
    const write = (name: string, catalog: object) =>
        files.writeTextFile(join(input.source, "locale", name), JSON.stringify(catalog));
    await files.mkdir(join(input.source, "locale"));
    await write("de.json", german);

    // read the catalog the build compiles and ships
    const build = await input.build(library.request);
    const shipped = await Catalog.read(build);

    // refuse a catalog named for another locale, then one of another package
    await write("de.json", { ...german, locale: "de-AT" });
    const misnamed = input.build(library.request);
    expect(await refusal(misnamed)).toEqual([
        "BAD_REQUEST",
        "CATALOG_INVALID",
        "catalog 'locale/de.json' is invalid: it does not parse",
    ]);
    await write("de.json", { ...german, package: "package-01996ab0-0000-7000-8000-0000000000aa" });
    const foreign = input.build(library.request);
    expect(await refusal(foreign)).toEqual([
        "BAD_REQUEST",
        "CATALOG_INVALID",
        "catalog 'locale/de.json' is invalid: it does not parse",
    ]);
    expect(shipped).toEqual([{ package: owner, locale: "de", messages: { a1: "Notiz" } }]);
});

test.concurrent("pin the compiler's version to the output it compiles", async ({ expect }) => {
    // build the library and digest its manifest, which has the digest of every file it outputs
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    await using input = await Fixture.open("library", environment);
    const build = await input.build(library.request);
    const output = await Digest.json(build.manifest, environment.crypto);

    // pair the version with its output: changed output takes a new COMPILER_VERSION and this digest
    expect({ version: COMPILER_VERSION, output }).toEqual({
        version: 14,
        output: "cf560349800045578c236401ef4a11359c5e56e7124a1a12677cfd57b265b2eb",
    });
});
