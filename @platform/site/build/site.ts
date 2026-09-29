import { type ApplicationOptions, buildPackage } from "@destack/build";
import { readDependencies, servePackage } from "@destack/build/local";
import { cp, mkdir, rm } from "node:fs/promises";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { join } from "node:path";
import { watchContent } from "./content.ts";
import { prerenderRoutes } from "../src/generated/prerender-routes.ts";

/** Absolute package directory for development and production builds. */
const directory = fileURLToPath(new URL("../", import.meta.url));

/** Build static pages or serve the same application with live updates. */
export async function run(command: string | undefined): Promise<void> {
    const application: ApplicationOptions = {
        kind: "web",
        ssr: { runtime: "bun", emit: false },
        minify: true,
        app: "src/app.tsx",
        document: "src/site/document.tsx",
        renderMode: "async" as const,
        prerender: { origin: "https://destack.sh", routes: prerenderRoutes, notFound: "/404" },
    };

    // retain the development server until the process receives a shutdown signal
    if (command === "dev") {
        await using development = await servePackage({
            directory,
            dependencies: await readDependencies(directory),
            application,
            server: { port: 3737, strictPort: true, watch: { ignored: ["**/.output/**"] } },
        });
        await using content = watchContent(directory, development.vite);
        development.vite.printUrls();
        await new Promise<void>((resolve) => {
            process.once("SIGINT", resolve);
            process.once("SIGTERM", resolve);
        });
    }
    // publish only the browser output, keeping inspection and source files private
    else if (command === "build") {
        const build = await buildPackage({
            directory,
            outputs: { website: application },
            dependencies: await readDependencies(directory),
        });
        const output = join(directory, ".output");
        await rm(output, { recursive: true, force: true });
        await mkdir(output, { recursive: true });
        await build.write(join(output, "package"));
        await mkdir(join(output, "public"), { recursive: true });
        const browser = build.manifest.outputs["website-browser"].directory;
        await cp(join(output, "package", browser), join(output, "public"), {
            recursive: true,
        });
    } else {
        throw new Error("Expected build or dev.");
    }
}
