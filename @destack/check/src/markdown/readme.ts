import { join } from "node:path";
import type { Environment } from "@destack/environment";
import { type ToolResult, runTool } from "../check/tool.ts";
import { CheckError } from "../error/index.ts";
import type { Toolchain } from "../toolchain/index.ts";

/** The directory a package's README listings are compiled in, with their own tsconfig. */
export const README_DIRECTORY = "node_modules/.cache/destack/readme";

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

    /** Typecheck a package's README listings in their own program, on the package's tsconfig. */
    async check(
        directory: string,
        toolchain: Toolchain,
        environment: Environment<"files" | "process">,
    ): Promise<ToolResult> {
        // read the README, which every package has
        const { files } = environment;
        const source = join(directory, "README.md");
        if (!(await files.exists(source))) {
            throw CheckError.of(
                "CONFIGURATION_INVALID",
                `package '${directory}' is invalid: it has no README.md`,
            );
        }

        // write each listing as a module of its own, replacing the last run's
        const folder = join(directory, README_DIRECTORY);
        const listings = Readme.listings(await files.readTextFile(source));
        await files.remove(folder, { recursive: true, force: true });
        await files.mkdir(folder, { recursive: true });
        const names = listings.map((listing, position) => `${position + 1}.${listing.language}`);
        for (const [position, listing] of listings.entries()) {
            await files.writeTextFile(
                join(folder, names[position] ?? ""),
                `${listing.body}export {};\n`,
            );
        }

        // compile them on the package's tsconfig, as rustdoc compiles doc listings apart from the crate
        const configuration = { extends: "../../../../tsconfig.json", include: [], files: names };
        await files.writeTextFile(join(folder, "tsconfig.json"), JSON.stringify(configuration));
        const typescript = toolchain.locate(["@destack/check", "typescript"], directory);

        return await runTool(
            join(typescript, "bin/tsc"),
            ["--project", folder],
            directory,
            environment,
        );
    },
};
