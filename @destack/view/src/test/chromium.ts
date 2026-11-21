import { CompilerPool, Workspace } from "@destack/build";
import { testStore } from "@destack/build/test";
import { Toolchain } from "@destack/check/toolchain";
import { present } from "@destack/schema";
import type { TestProject } from "@destack/test";
import { testHost } from "@destack/test/host";
import { chromium } from "playwright";
import { DevelopmentServer } from "../build/server.ts";

/** Start the examples server and Chromium of a run. */
export default async function setup(project: TestProject): Promise<() => Promise<void>> {
    // serve the package's examples from its own workspace
    const root = project.config.root;
    const environment = testHost();
    const started = new AsyncDisposableStack();
    const store = started.use(await testStore());
    const compilers = started.use(
        await CompilerPool.open(store, async () => Toolchain.workspace()),
    );
    const workspace = await (await Workspace.open(root, compilers)).package();
    const server = started.use(
        await DevelopmentServer.examples({ workspace, environment, server: { port: 0 } }),
    );
    const origin = present(server.vite.resolvedUrls?.local[0], "the examples server's address");

    // launch the Chromium every scenario of the run connects to
    const browser = await chromium.launchServer({ headless: true });
    started.defer(async () => browser.close());

    // import each scenario module once to warm the server
    const warmed = await chromium.connect(browser.wsEndpoint());
    const page = await warmed.newPage();
    for (const file of project.config.include) {
        const query = new URLSearchParams({ scenario: `/${file}` });
        await page.goto(`${origin}?${query.toString()}`, { waitUntil: "networkidle" });
    }
    await page.reload({ waitUntil: "networkidle" });
    await warmed.close();
    project.provide("chromium", {
        endpoint: browser.wsEndpoint(),
        origin: origin.replace(/\/$/u, ""),
        root,
    });

    return async () => started.disposeAsync();
}
