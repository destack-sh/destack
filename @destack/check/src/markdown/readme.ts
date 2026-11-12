import { dirname, join } from "node:path";
import type { FileSystem } from "@destack/environment";
import { CheckError } from "../error/index.ts";

/** The module a package's typecheck compiles for its README, which its tsconfig lists. */
export const README_MODULE = "node_modules/.cache/destack/readme/README.ts";

/** A fenced TypeScript listing: its language and its body. */
const LISTING = /^```(ts|tsx)(?: no_run)?\n([\s\S]*?)^```$/gmu;

/** A README's TypeScript listing: its language and its body. */
export interface ReadmeListing {
    /** The listing's language, `ts` or `tsx`. */
    readonly language: "ts" | "tsx";
    /** The listing's code. */
    readonly body: string;
}

/** A package README's TypeScript listings, each compiled as a module of its own. */
export const Readme = {
    /** Read a README's TypeScript listings in order. */
    listings(markdown: string): ReadmeListing[] {
        return [...markdown.matchAll(LISTING)].map(([, language, body = ""]) => ({
            language: language === "tsx" ? "tsx" : "ts",
            body,
        }));
    },

    /** Write a package's README listings as the modules its typecheck compiles. */
    async write(directory: string, files: FileSystem): Promise<void> {
        // read the README, which every package has
        const source = join(directory, "README.md");
        if (!(await files.exists(source))) {
            throw CheckError.of(
                "CONFIGURATION_INVALID",
                `package '${directory}' is invalid: it has no README.md`,
            );
        }

        // write each listing as a module of its own, replacing the last run's
        const index = join(directory, README_MODULE);
        const folder = dirname(index);
        const listings = Readme.listings(await files.readTextFile(source));
        if (await files.exists(folder)) {
            await files.remove(folder, { recursive: true });
        }
        await files.mkdir(folder, { recursive: true });
        const modules = listings.map((listing, position) => ({
            name: `${position + 1}.${listing.language}`,
            text: `${listing.body}export {};\n`,
        }));
        for (const { name, text } of modules) {
            await files.writeTextFile(join(folder, name), text);
        }

        // write the module importing them all, which the package's tsconfig lists
        const imports = modules.map(({ name }) => `import "./${name}";\n`);
        await files.writeTextFile(index, imports.join(""));
    },
};
