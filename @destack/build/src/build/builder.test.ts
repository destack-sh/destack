import { expect, test } from "@destack/test";
import { cp, mkdtemp, readFile, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { buildPackage, PackageBuild, PackageBuilder } from "../index.ts";
import { request } from "../../tests/fixture/library/request.ts";
import { readDependencies } from "../local/index.ts";
import { linkDependencies } from "../source/index.ts";
import * as resource from "../../tests/fixture/resource/request.ts";
import { Fixture, expectBuild, expectFiles } from "../../tests/fixture.ts";
import { requests } from "../../tests/fixture/web/request.ts";
import { formatSource } from "@destack/check";
import { schema } from "@destack/schema";
import { DeclarationDescription } from "@destack/package/inspect";

test("rebuild a web application with emitted assets and restore every output", async () => {
    await using fixture = await Fixture.open("web");
    const file = join(fixture.source, "src/app.tsx");
    const original = await readFile(file, "utf8");

    // declare the asset imports used by this standalone application
    const declaration = join(fixture.source, "src/assets.d.ts");
    await writeFile(
        declaration,
        await formatSource(
            declaration,
            '/** SVG assets imported as URLs. */\ndeclare module "*.svg?url" { const url: string; export default url; }',
        ),
    );

    // retain two distinct emitted assets through the application's public render
    for (const name of ["first", "second"]) {
        await writeFile(
            join(fixture.source, `src/${name}.svg`),
            `<svg xmlns="http://www.w3.org/2000/svg"><!--${name.repeat(1500)}--></svg>`,
        );
    }
    const source = await formatSource(
        file,
        'import first from "./first.svg?url";\nimport second from "./second.svg?url";\n' +
            original.replace(
                "<main {...theme}>",
                "<main {...theme}><img src={first} /><img src={second} />",
            ),
    );
    await writeFile(file, source);
    await using builder = await PackageBuilder.start(fixture.source);
    const options = { dependencies: fixture.dependencies, outputs: { website: requests.browser } };

    // compare the full distribution before and after an application edit
    await using first = await builder.build(options);
    await using repeated = await builder.build(options);
    expect(repeated.manifest).toEqual(first.manifest);
    await expectFiles(repeated, first);
    await writeFile(file, source.replace("Hello Destack", "Edited Destack"));
    await using edited = await builder.build(options);
    expect(edited.manifest).not.toEqual(first.manifest);
    await writeFile(file, source);
    await using restored = await builder.build(options);
    expect(restored.manifest).toEqual(first.manifest);
    await expectFiles(restored, first);
});

test("reinspect edited declaration helpers in a retained compiler", async () => {
    const fixture = new URL("../../tests/fixture/resource/source/", import.meta.url);
    const directory = await mkdtemp(join(tmpdir(), "destack-declaration-edit-"));
    try {
        // evaluate an ordinary imported title module and retain the complete initial output
        await cp(fixture, directory, {
            recursive: true,
            filter: (path) => !path.endsWith("/node_modules") && !path.endsWith("\\node_modules"),
        });
        await linkDependencies(fileURLToPath(fixture), directory);
        const file = join(directory, "src/database.ts");
        const source = await readFile(file, "utf8");
        await writeFile(
            file,
            'import { title } from "./title.ts";\n' +
                source.replace('text("title").notNull()', "text(title()).notNull()"),
        );
        const titleModule = join(directory, "src/title.ts");
        const original =
            '/** Return the column name. */\nexport function title(): string {\n    return "title";\n}\n';
        await writeFile(titleModule, original);
        const dependencies = await readDependencies(fileURLToPath(fixture));
        await using builder = await PackageBuilder.start(directory);
        await using first = await builder.build({ dependencies, ...resource.request });

        // reload the title module and preserve declaration logs outside the result stream
        await writeFile(
            titleModule,
            'console.info("inspecting title");\n/** Return the column name. */\nexport function title(): string {\n    return "heading";\n}\n',
        );
        await using edited = await builder.build({ dependencies, ...resource.request });
        const baseline = (
            await first.reader.domain("db", schema.array(DeclarationDescription))
        ).find((declaration) => declaration.kind === "database-schema")!;
        const actual = (
            await edited.reader.domain("db", schema.array(DeclarationDescription))
        ).find((declaration) => declaration.kind === "database-schema")!;
        const expected = structuredClone(baseline);
        for (const dialect of ["sqlite", "postgresql"]) {
            const schema = expected.description[dialect] as {
                tables: { columns: { name: string }[] }[];
            };
            schema.tables[0].columns[1].name = "heading";
        }
        expect(actual).toEqual(expected);

        // restore the title module and compare every distributed byte and the full manifest
        await writeFile(titleModule, original);
        await using restored = await builder.build({ dependencies, ...resource.request });
        expect(restored.manifest).toEqual(first.manifest);
        await expectFiles(restored, first);

        // await the same shutdown through concurrent callers and the enclosing using scope
        await Promise.all([builder[Symbol.asyncDispose](), builder[Symbol.asyncDispose]()]);
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("rebuild edited source, reject incompatible APIs, restore distributed output", async () => {
    const fixture = new URL("../../tests/fixture/library/", import.meta.url);
    const directory = await mkdtemp(join(tmpdir(), "destack-build-library-"));
    const source = join(directory, "source");
    const destination = join(directory, "build");

    try {
        // compile a real package and compare its complete distributed output
        await cp(new URL("source/", fixture), source, { recursive: true });
        await using builder = await PackageBuilder.start(source);
        await using build = await builder.build(request);
        await expectBuild(build, new URL("expected/", fixture));

        // rebuild unchanged, edited, and restored source with the same compiler
        await using unchanged = await builder.build(request);
        expect(unchanged.manifest).toEqual(build.manifest);
        await expectFiles(unchanged, build);
        const notePath = join(source, "src/note.ts");
        const note = await readFile(notePath, "utf8");
        await writeFile(
            notePath,
            note
                .replace("complete: boolean", "complete: true")
                .replace("complete: false", "complete: true"),
        );
        await using edited = await builder.build(request);
        const editDirectory = join(directory, "edited");
        await edited.write(editDirectory);
        const editedModule = await import(
            pathToFileURL(join(editDirectory, edited.manifest.outputs.library.exports["."])).href
        );
        expect(editedModule.createNote("Edited")).toEqual({ title: "Edited", complete: true });
        await writeFile(notePath, note);
        await using restoredSource = await builder.build(request);
        expect(restoredSource.manifest).toEqual(build.manifest);
        await expectFiles(restoredSource, build);

        // reject an API unavailable in the selected runtime, then accept restored source
        await writeFile(
            notePath,
            note + "\n/** The page title. */\nexport const page = document.title;\n",
        );
        await expect(
            builder.build({
                dependencies: {},
                outputs: {
                    library: { ...request.outputs.library, runtime: "workerd" },
                },
            }),
        ).rejects.toMatchObject({
            code: "BUILD_FAILED",
            message: `Unsupported workerd API: document.title at src/note.ts:${note.length + 44}`,
        });

        // accept browser index signatures through the runtime's actual type declarations
        const browserSource =
            note +
            "\n/** The page theme. */\nexport const page = document.documentElement.dataset.theme;\n";
        await writeFile(notePath, browserSource);
        await using browser = await builder.build({
            dependencies: {},
            outputs: {
                library: {
                    ...request.outputs.library,
                    target: "browser",
                    runtime: "browser",
                },
            },
        });
        expect(await readFile(join(browser.directory, "src/note.ts"), "utf8")).toBe(browserSource);

        // reject server functions before their bodies can enter a browser output
        await writeFile(notePath, '"use server";\n' + note);
        const rejected = builder.build({
            dependencies: {},
            outputs: {
                library: {
                    kind: "module",
                    target: "browser",
                    runtime: "browser",
                },
            },
        });
        const filename = await realpath(notePath);
        const diagnostic = await rejected.then(
            () => {
                throw new Error("expected server-function rejection");
            },
            (error) => ({ code: error.code, message: error.message.split("\n    at ")[0] }),
        );
        expect(diagnostic).toEqual({
            code: "BUILD_FAILED",
            message: `Build failed with 1 error:\n\n[plugin destack-framework] ${filename}\nBuildError: Solid server functions are unsupported: ${filename}:0`,
        });
        await writeFile(notePath, note);

        // remove source access before loading the distributed package
        await build.write(destination);
        await rm(source, { recursive: true });
        const restored = await PackageBuild.read(destination);
        expect(restored.manifest).toEqual(build.manifest);

        // preserve an existing distribution when the caller repeats a write
        await expect(build.write(destination)).rejects.toMatchObject({ code: "EEXIST" });
        expect((await PackageBuild.read(destination)).manifest).toEqual(build.manifest);

        // leave no published manifest when an output cannot be written
        const failed = join(directory, "failed");
        await rm(join(build.directory, build.manifest.outputs.library.exports["."]));
        await expect(build.write(failed)).rejects.toMatchObject({ code: "ENOENT" });
        await expect(readFile(join(failed, "manifest.json"))).rejects.toMatchObject({
            code: "ENOENT",
        });
        const output = restored.manifest.outputs.library;
        const module = await import(pathToFileURL(join(destination, output.exports["."])).href);
        expect(module.createNote("A note")).toEqual({ title: "A note", complete: false });

        // discard temporary compiler output while preserving the retained distribution
        await build[Symbol.asyncDispose]();
        await expect(readFile(join(build.directory, "manifest.json"))).rejects.toMatchObject({
            code: "ENOENT",
        });
        await restored[Symbol.asyncDispose]();
        expect((await PackageBuild.read(destination)).manifest).toEqual(restored.manifest);
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("reject invalid outputs and recover the retained compiler", async () => {
    const source = fileURLToPath(new URL("../../tests/fixture/library/source/", import.meta.url));
    await using builder = await PackageBuilder.start(source);
    await expect(builder.build({ dependencies: {}, outputs: {} })).rejects.toMatchObject({
        code: "BUILD_FAILED",
        message: "A build requires at least one output.",
    });
    await expect(
        builder.build({
            dependencies: {},
            outputs: {
                "website-browser": request.outputs.library,
                website: { kind: "web", ssr: false },
            },
        }),
    ).rejects.toMatchObject({
        code: "BUILD_FAILED",
        message: "Duplicate output name: website-browser",
    });

    // use the same compiler successfully after rejected requests
    await using build = await builder.build(request);
    const expected = JSON.parse(
        await readFile(
            new URL("../../tests/fixture/library/expected/manifest.json", import.meta.url),
            "utf8",
        ),
    );
    expect(build.manifest).toEqual(expected);
});

test("terminate compilation on cancellation and deadline", async () => {
    const directory = fileURLToPath(
        new URL("../../tests/fixture/library/source/", import.meta.url),
    );
    const controller = new AbortController();
    await using builder = await PackageBuilder.start(directory);
    const pending = builder.build({ ...request, signal: controller.signal });
    controller.abort();
    await expect(pending).rejects.toMatchObject({
        code: "BUILD_FAILED",
        message: "Build cancelled.",
    });
    await expect(buildPackage({ ...request, directory, timeout: 1 })).rejects.toMatchObject({
        code: "BUILD_FAILED",
        message: "Build exceeded 1 ms.",
    });
});
