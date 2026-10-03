import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

/** The name of the toolchain directory beside a standalone executable. */
const TOOLCHAIN = "toolchain";

/** Whether this process is a standalone Bun executable. */
const IS_STANDALONE = "Bun" in globalThis && Bun.isStandaloneExecutable;

/** The tools on disk: the toolchain directory beside a standalone executable, or the packages a workspace installed. */
export const Toolchain = {
    /** The toolchain directory beside a standalone executable, absent when running from a workspace. */
    directory: IS_STANDALONE ? join(dirname(process.execPath), TOOLCHAIN) : undefined,

    /** The suffix naming this platform's native addon packages, as napi-rs names them for glibc on Linux. */
    platform: `${process.platform}-${process.arch}${process.platform === "linux" ? "-gnu" : ""}`,

    /** Name the toolchain directory beside an executable. */
    beside(executable: string): string {
        return join(dirname(executable), TOOLCHAIN);
    },

    /** Find an installed package's directory among the toolchain's packages, else through a chain of packages from a package directory, each depending on the next. */
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
