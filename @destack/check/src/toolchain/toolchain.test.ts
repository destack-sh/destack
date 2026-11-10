import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { schema } from "@destack/schema";
import { Toolchain } from "./toolchain.ts";

/** This package's directory, which installed the tools in the workspace. */
const PACKAGE = fileURLToPath(new URL("../..", import.meta.url));

/** The name field of a package.json. */
const PackageName = schema.object({ name: schema.string() }).strip();

/** A toolchain manifest's digest, which names its directory. */
const DIGEST = "c0ffee".padEnd(64, "0");

test("resolve each package of a chain from the one before in a workspace", async () => {
    // locate the type-aware linter and its native package through the linter that installed it
    const { files, host } = testHost();
    const toolchain = Toolchain.workspace();
    const native = `@oxlint-tsgolint/${host.os}-${host.architecture}`;
    const linter = toolchain.locate(["oxlint-tsgolint"], PACKAGE);
    const checker = toolchain.locate(["oxlint-tsgolint", native], PACKAGE);
    const name = async (directory: string) =>
        PackageName.parse(JSON.parse(await files.readTextFile(join(directory, "package.json"))))
            .name;

    expect({
        directory: toolchain.directory,
        linter: await name(linter),
        checker: await name(checker),
        lint: toolchain.lint(),
    }).toEqual({
        directory: undefined,
        linter: "oxlint-tsgolint",
        checker: native,
        lint: fileURLToPath(new URL("../lint/index.ts", import.meta.url)),
    });
});

test("resolve the last package of a chain among a toolchain directory's flat packages", async () => {
    // lay out a toolchain with one native package
    const { files } = testHost();
    const root = await files.realPath(await files.makeTemporaryDirectory("toolchain-"));
    const directory = join(root, DIGEST);
    const native = join(directory, "node_modules", "@oxlint-tsgolint", "darwin-arm64");
    await files.mkdir(native, { recursive: true });
    await files.writeTextFile(join(native, "package.json"), "{}");

    // resolve from the toolchain, passing over the workspace directory
    const toolchain = Toolchain.at(directory);
    try {
        expect({
            directory: toolchain.directory,
            checker: toolchain.locate(
                ["oxlint-tsgolint", "@oxlint-tsgolint/darwin-arm64"],
                PACKAGE,
            ),
            lint: toolchain.lint(),
        }).toEqual({
            directory,
            checker: native,
            lint: join(directory, "lint.js"),
        });
    } finally {
        await files.remove(root, { recursive: true });
    }
});
