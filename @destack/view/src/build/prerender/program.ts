import { join } from "node:path";
import type { Environment } from "@destack/environment";
import { BunEnvironment } from "@destack/environment/bun";
import type { PageAssets, StaticRoutes } from "../../static/page.ts";
import { planRequests, RENDER_PLAN_FILE, RENDERED_FILE, RenderPlan } from "./prerender.ts";

/** A static view's server entry: its routes and the answer to each request. */
interface ServerModule {
    /** List the view's routes. */
    readonly routes: () => Promise<StaticRoutes>;
    /** Answer a request, linking a page's assets. */
    readonly respond: (request: Request, assets: PageAssets) => Promise<Response>;
}

/** Render the plan in the directory the parent passes, with its server module. */
async function renderPlan(environment: Environment<"files" | "process">): Promise<void> {
    // read the plan the prerender wrote into the directory it passes
    const { files, process } = environment;
    const [directory] = process.arguments;
    if (directory === undefined) {
        throw new TypeError("the renderer needs the directory of its render plan");
    }
    const plan = RenderPlan.parse(
        JSON.parse(await files.readTextFile(join(directory, RENDER_PLAN_FILE))),
    );

    // plan the view's routes from its server module
    const server: unknown = await import(plan.module);
    if (!isServerModule(server)) {
        throw new TypeError(
            `the server module exports no routes and respond functions: '${plan.module}'`,
        );
    }
    const requests = planRequests(plan.origin, await server.routes());
    await files.writeTextFile(join(directory, RENDERED_FILE), JSON.stringify(requests));
    await process.stdout.write(`${JSON.stringify({ planned: requests.length })}\n`);

    // write each body and its response, reporting each completed one on the standard output
    for (const [index, request] of requests.entries()) {
        const response = await server.respond(new Request(request.url), plan.assets);
        const body = new Uint8Array(await response.arrayBuffer());
        await files.writeFile(join(directory, `${index}.body`), body);
        await files.writeTextFile(
            join(directory, `${index}.json`),
            JSON.stringify({
                status: response.status,
                headers: Object.fromEntries(response.headers),
            }),
        );
        await process.stdout.write(`${JSON.stringify({ rendered: index })}\n`);
    }
}

/** Report whether a module exports a static view's routes and respond functions. */
function isServerModule(module: unknown): module is ServerModule {
    return (
        typeof module === "object" &&
        module !== null &&
        "routes" in module &&
        typeof module.routes === "function" &&
        "respond" in module &&
        typeof module.respond === "function"
    );
}

// render only as the process's entry, as importing the export runs nothing
if (import.meta.main) {
    await renderPlan(BunEnvironment.system);
}
