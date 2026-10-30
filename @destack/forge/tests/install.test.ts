import { expect, onTestFinished, test } from "@destack/test";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { PackageBuild } from "@destack/build";
import { schema } from "@destack/schema";
import { fixtureBuild } from "./fixture/build.ts";
import { readJson, run } from "./fixture/package.ts";
import { testHost } from "@destack/test/host";
import { TestForge } from "../src/test/index.ts";

/** Each package manager with how it installs a project and runs the installed code. */
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
        install: (cache: string) => [
            testHost().process.executable,
            "install",
            "--cache-dir",
            cache,
        ],
        run: [testHost().process.executable, "run.js"],
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
        // publish a package and one built against it from the registry, served on a loopback port
        const forge = await TestForge.listen();
        const answer = await forge.publish((await fixtureBuild("answer")).reader);
        const greeting = await forge.publish((await fixtureBuild("greeting")).reader);

        // install the dependent package into a consumer project through the registry, which answers from the world
        const { files } = testHost();
        const directory = await files.makeTemporaryDirectory("destack-forge-install-");
        onTestFinished(() => files.remove(directory, { recursive: true }));
        const consumer = join(directory, "consumer");
        await files.copy(fileURLToPath(new URL("./fixture/consumer/", import.meta.url)), consumer);
        await files.writeTextFile(join(consumer, ".npmrc"), await forge.npmrc("@example"));
        await files.mkdir(join(directory, "cache"), { recursive: true });
        const selected = MANAGERS[manager];
        const { scheduler } = forge.home.simulation;
        await scheduler.track(
            run(selected.install(join(directory, "cache", manager)), consumer),
            "peer",
        );

        // run it, and restore both original builds from what was installed, finding each package as Node resolves it
        const greetingPath = await installed(consumer, "@example/greeting");
        const answerPath = await installed(greetingPath, "@example/answer");
        expect({
            output: await run(selected.run, consumer),
            answer: (await PackageBuild.read(join(answerPath, "build"), testHost())).manifest,
            greeting: (await PackageBuild.read(join(greetingPath, "build"), testHost())).manifest,
            dependencies: DECLARATION.parse(await readJson(join(greetingPath, "package.json")))
                .dependencies,
        }).toEqual({
            output: "hello world, the answer is 42\n",
            answer: answer.reader.manifest,
            greeting: greeting.reader.manifest,
            dependencies: { "@example/answer": "2026.9.0" },
        });
    },
);

/** Find an installed package's directory through each enclosing node_modules. */
async function installed(from: string, name: string): Promise<string> {
    const { files } = testHost();
    for (let directory = await files.realPath(from); ; directory = dirname(directory)) {
        // take the package from this directory's node_modules, else look in the enclosing one
        const candidate = join(directory, "node_modules", name);
        if (await files.exists(candidate)) {
            return files.realPath(candidate);
        } else if (dirname(directory) === directory) {
            throw new Error(`${name} is not installed for ${from}`);
        }
    }
}
