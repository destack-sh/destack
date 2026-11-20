import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { schema } from "@destack/schema";
import { Toolchain } from "./toolchain.ts";

/** The name field of a package.json. */
const PackageName = schema.object({ name: schema.string() }).strip();

/** A toolchain manifest's digest, which names its directory. */
const DIGEST = "c0ffee".padEnd(64, "0");

test("resolve each package of a chain from the one before in a workspace", async () => {
    // locate the formatter and its native binding through the formatter that installed it
    const { files, host } = testHost();
    const toolchain = Toolchain.workspace();
    const native = `@oxfmt/binding-${host.os}-${host.architecture}`;
    const formatter = toolchain.locate(["oxfmt"], Toolchain.packageDirectory());
    const binding = toolchain.locate(["oxfmt", native], Toolchain.packageDirectory());
    const name = async (directory: string) =>
        PackageName.parse(JSON.parse(await files.readTextFile(join(directory, "package.json"))))
            .name;

    expect({
        directory: toolchain.directory,
        formatter: await name(formatter),
        binding: await name(binding),
        lint: toolchain.lint(),
    }).toEqual({
        directory: undefined,
        formatter: "oxfmt",
        binding: native,
        lint: fileURLToPath(new URL("../lint/index.ts", import.meta.url)),
    });
});

test("resolve the last package of a chain among a toolchain directory's flat packages", async () => {
    // lay out a toolchain with one native package
    const { files } = testHost();
    const root = await files.realPath(await files.makeTemporaryDirectory("toolchain-"));
    const directory = join(root, DIGEST);
    const native = join(directory, "node_modules", "@oxfmt", "binding-darwin-arm64");
    await files.mkdir(native, { recursive: true });
    await files.writeTextFile(join(native, "package.json"), "{}");

    // resolve from the toolchain, passing over the workspace directory
    const toolchain = Toolchain.at(directory);
    try {
        expect({
            directory: toolchain.directory,
            binding: toolchain.locate(
                ["oxfmt", "@oxfmt/binding-darwin-arm64"],
                Toolchain.packageDirectory(),
            ),
            lint: toolchain.lint(),
        }).toEqual({
            directory,
            binding: native,
            lint: join(directory, "lint.js"),
        });
    } finally {
        await files.remove(root, { recursive: true });
    }
});
