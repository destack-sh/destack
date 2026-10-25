import { join } from "node:path";
import type { Environment } from "@destack/environment";
import { BunEnvironment } from "@destack/environment/bun";
import { WebError } from "../../error/error.ts";
import { RENDER_PLAN_FILE, RenderPlan } from "./prerender.ts";
import { type PageHandler, renderPages } from "./render.ts";

/** Render the plan in the directory the parent passes, with its server module's handler. */
async function renderPlan(environment: Environment<"files" | "process">): Promise<void> {
    // read the plan the prerender wrote into the directory it passes
    const [directory] = environment.process.arguments;
    if (directory === undefined) {
        throw new TypeError("the renderer needs the directory of its render plan");
    }
    const text = await environment.files.readTextFile(join(directory, RENDER_PLAN_FILE));
    const plan = RenderPlan.parse(JSON.parse(text));

    // render each planned URL with the server module's handler
    const server: unknown = await import(plan.module);
    if (!isServerModule(server)) {
        throw WebError.of(
            "SSR_REQUIRED",
            `the server module exports no handleRequest function: ${plan.module}`,
        );
    }
    await renderPages(server.handleRequest, plan.urls, directory, environment);
}

/** Report whether a module exports a page handler as its handleRequest. */
function isServerModule(module: unknown): module is { readonly handleRequest: PageHandler } {
    return (
        typeof module === "object" &&
        module !== null &&
        "handleRequest" in module &&
        typeof module.handleRequest === "function"
    );
}

await renderPlan(BunEnvironment.system);
