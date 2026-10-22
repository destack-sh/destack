import { createRequire } from "node:module";
import { dirname, join, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { BunProcess } from "../bun/process.ts";

/** The name of the toolchain directory beside a standalone executable. */
const TOOLCHAIN = "toolchain";

/** The tools on disk. */
export const Toolchain = {
    /** The toolchain directory of a standalone executable, absent in a workspace. */
    get directory(): string | undefined {
        return BunProcess.isStandalone ? Toolchain.of(process.execPath) : undefined;
    },

    /** The suffix of this platform's native addon packages. */
    platform: `${process.platform}-${process.arch}${process.platform === "linux" ? "-gnu" : ""}`,

    /** Find an executable's toolchain: in its macOS application's resources, else beside it. */
    of(executable: string): string {
        // read a macOS application's toolchain from its bundle's resources for code signing
        const segments = executable.split(sep);
        const contents = segments.lastIndexOf("Contents");
        if (segments[contents - 1]?.endsWith(".app") === true) {
            return join(segments.slice(0, contents + 1).join(sep), "Resources", TOOLCHAIN);
        }

        // read the toolchain beside any other executable
        return join(dirname(executable), TOOLCHAIN);
    },

    /** Find an installed package's directory among the toolchain's packages. */
    locate(chain: readonly string[], from: string): string {
        // find the last package among the toolchain's, else each package from the one before
        const names = Toolchain.directory === undefined ? chain : chain.slice(-1);
        let directory = Toolchain.directory ?? from;
        for (const name of names) {
            const require = createRequire(join(directory, "package.json"));
            directory = dirname(require.resolve(`${name}/package.json`));
        }

        return directory;
    },

    /** Find the module with this package's lint rules: the toolchain's bundle, or the source. */
    lint(): string {
        return Toolchain.directory === undefined
            ? fileURLToPath(new URL("../lint/index.ts", import.meta.url))
            : join(Toolchain.directory, "lint.js");
    },
};
