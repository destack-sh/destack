import { expect, test } from "@destack/test";
import { cp, mkdir, realpath, stat, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { PackageBuild } from "@destack/build";
import { schema } from "@destack/schema";
import { readJson, PackageFixture, run } from "./fixture/package.ts";

/** Each package manager with how it installs a project and runs the installed code, deno accepting the versions just published. */
const MANAGERS = {
    npm: {
        install: (cache: string) => [
            "npm",
            "install",
            "--ignore-scripts",
            "--no-audit",
            "--no-fund",
            "--cache",
            cache,
        ],
        run: ["node", "run.js"],
    },
    bun: {
        install: (cache: string) => [process.execPath, "install", "--cache-dir", cache],
        run: [process.execPath, "run.js"],
    },
    deno: {
        install: () => ["deno", "install", "--node-modules-dir=auto", "--minimum-dependency-age=0"],
        run: ["deno", "run", "--allow-read", "--allow-env", "--node-modules-dir=auto", "run.js"],
    },
} as const;

/** The package managers each install runs with. */
const MANAGER_NAMES: readonly (keyof typeof MANAGERS)[] = ["npm", "bun", "deno"];

/** The installed declaration fields the install checks. */
const DECLARATION = schema.object({ dependencies: schema.json() }).strip();

test.for(MANAGER_NAMES)(
    "install Destack packages and their Destack dependencies from the registry with %s",
    { timeout: 120_000 },
    async (manager) => {
        // publish a package and one built against it from the registry
        await using forge = await PackageFixture.open("sqlite");
        const answer = await forge.publish("answer");
        const greeting = await forge.publish("greeting");

        // install the dependent package into a consumer project through the registry
        const consumer = join(forge.directory, "consumer");
        await cp(fileURLToPath(new URL("./fixture/consumer/", import.meta.url)), consumer, {
            recursive: true,
        });
        await writeFile(join(consumer, ".npmrc"), forge.npmrc());
        await mkdir(join(forge.directory, "cache"), { recursive: true });
        const selected = MANAGERS[manager];
        await run(selected.install(join(forge.directory, "cache", manager)), consumer);

        // run it, and restore both original builds from what was installed, finding each package as Node resolves it
        const greetingPath = await installed(consumer, "@example/greeting");
        const answerPath = await installed(greetingPath, "@example/answer");
        expect({
            output: await run(selected.run, consumer),
            answer: (await PackageBuild.read(join(answerPath, "build"))).manifest,
            greeting: (await PackageBuild.read(join(greetingPath, "build"))).manifest,
            dependencies: DECLARATION.parse(await readJson(join(greetingPath, "package.json")))
                .dependencies,
        }).toEqual({
            output: "hello world, the answer is 42\n",
            answer: answer.build.manifest,
            greeting: greeting.build.manifest,
            dependencies: { "@example/answer": "2026.9.0" },
        });
    },
);

/** Find the directory of an installed package as Node resolves it from a directory: through each enclosing node_modules. */
async function installed(from: string, name: string): Promise<string> {
    for (let directory = await realpath(from); ; directory = dirname(directory)) {
        // take the package from this directory's node_modules, else look in the enclosing one
        const candidate = join(directory, "node_modules", name);
        if (await exists(candidate)) {
            return realpath(candidate);
        } else if (dirname(directory) === directory) {
            throw new Error(`${name} is not installed for ${from}`);
        }
    }
}

/** Decide whether a path exists. */
async function exists(path: string): Promise<boolean> {
    return stat(path).then(
        () => true,
        () => false,
    );
}
