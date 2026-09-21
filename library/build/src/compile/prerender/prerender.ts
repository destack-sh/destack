import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { pathToFileURL } from "node:url";
import { PackagePath } from "@destack/package/file";
import { BuildError } from "../../error/index.ts";
import { spawn } from "node:child_process";
import { createInterface } from "node:readline";

/** Requests rendered to static HTML during a build. */
export interface PrerenderOptions {
    /** Absolute origin used for build-time requests. */
    origin: string;
    /** URL paths to render, including a leading slash. */
    routes: readonly string[];
    /** Route returning 404 to publish as 404.html. */
    notFound?: string;
    /** Maximum time per request, including its body, in milliseconds. */
    timeout?: number;
}

/** Render explicit public routes into a browser output directory. */
export async function prerender(
    module: string,
    options: PrerenderOptions,
): Promise<Map<string, Uint8Array<ArrayBuffer>>> {
    const origin = new URL(options.origin);
    if (origin.origin !== options.origin || !["http:", "https:"].includes(origin.protocol)) {
        throw new BuildError("BUILD_FAILED", `Expected an HTTP origin: ${options.origin}`);
    }
    const files = new Map<string, Uint8Array<ArrayBuffer>>();

    // reject ambiguous URLs before executing application code
    const routes = [...options.routes, ...(options.notFound ? [options.notFound] : [])];
    const requests = routes.map((route) => {
        const url = new URL(route, origin);
        if (url.origin !== origin.origin || url.pathname !== route || url.search || url.hash) {
            throw new BuildError("BUILD_FAILED", `Expected a canonical URL path: ${route}`);
        }
        const isNotFound = route === options.notFound;
        const path = isNotFound
            ? "404.html"
            : PackagePath.parse(
                  `${decodeURIComponent(route).replace(/^\/|\/$/g, "")}/index.html`.replace(
                      /^\//,
                      "",
                  ),
              );

        return { url, path, status: isNotFound ? 404 : 200 };
    });
    if (new Set(requests.map((request) => request.path)).size !== requests.length) {
        throw new BuildError("BUILD_FAILED", "Prerender routes produce duplicate files.");
    }

    // reuse the restricted renderer across requests
    const directory = await mkdtemp(join(tmpdir(), "destack-render-"));
    try {
        const program = join(directory, "render.js");
        const outputs = requests.flatMap((_, index) => [
            join(directory, `${index}.html`),
            join(directory, `${index}.json`),
        ]);
        await writeFile(
            program,
            [
                `import * as server from ${JSON.stringify(pathToFileURL(module).href)};`,
                `import { renderPages } from ${JSON.stringify(
                    new URL("./worker.ts", import.meta.url).href,
                )};`,
                `await renderPages(server.handleRequest, ${JSON.stringify(
                    requests.map((request) => request.url.href),
                )}, ${JSON.stringify(directory)});`,
            ].join("\n"),
        );
        await runProgram(program, outputs, options.timeout ?? 10_000, requests.length);
        for (const [index, request] of requests.entries()) {
            try {
                const body = outputs[index * 2];
                const metadata = outputs[index * 2 + 1];
                const response = JSON.parse(await readFile(metadata, "utf8"));
                if (
                    response.status !== request.status ||
                    !response.headers["content-type"]?.startsWith("text/html")
                ) {
                    throw new BuildError(
                        "BUILD_FAILED",
                        `Expected an HTML response: ${request.url.pathname} (${response.status})`,
                    );
                }
                if (
                    response.headers["set-cookie"] ||
                    /private|no-store/i.test(response.headers["cache-control"] ?? "") ||
                    response.headers.vary
                ) {
                    throw new BuildError(
                        "BUILD_FAILED",
                        `Cannot prerender a personalized response: ${request.url.pathname}`,
                    );
                }
                files.set(request.path, new Uint8Array(await readFile(body)));
            } catch (cause) {
                throw new BuildError("BUILD_FAILED", `Prerender failed: ${request.url.pathname}`, {
                    cause,
                });
            }
        }
    } finally {
        await rm(directory, { recursive: true });
    }

    return files;
}

/** Render pages with restricted host access and a deadline between completed responses. */
export async function runProgram(
    program: string,
    outputs: readonly string[],
    timeout: number,
    count: number,
): Promise<void> {
    const directory = dirname(program);
    const child = spawn(
        "deno",
        [
            "run",
            "--no-config",
            "--no-lock",
            "--no-remote",
            "--no-npm",
            "--no-prompt",
            "--deny-read",
            "--deny-net",
            "--deny-env",
            "--deny-run",
            "--deny-ffi",
            "--deny-sys",
            `--allow-write=${outputs.join(",")}`,
            program,
        ],
        {
            cwd: directory,
            env: { PATH: process.env.PATH, DENO_DIR: join(directory, "cache"), NO_COLOR: "1" },
            stdio: ["ignore", "pipe", "pipe"],
        },
    );

    await new Promise<void>((resolve, reject) => {
        let completed = 0;
        let diagnostics = "";
        let failure: Error | undefined;
        const expire = () => {
            failure = new BuildError(
                "BUILD_FAILED",
                `Prerender response ${completed} exceeded ${timeout} ms.`,
            );
            child.kill("SIGKILL");
        };
        let timer = setTimeout(expire, timeout);

        // extend the deadline only after the next complete response
        const progress = createInterface({ input: child.stdout });
        progress.on("line", (line) => {
            if (completed < count && line === JSON.stringify({ rendered: completed })) {
                completed++;
                clearTimeout(timer);
                timer = setTimeout(expire, timeout);
            }
        });
        child.stderr.setEncoding("utf8");
        child.stderr.on("data", (chunk: string) => {
            diagnostics += chunk;
            if (diagnostics.length > 1_048_576) {
                failure = new BuildError("BUILD_FAILED", "Prerender diagnostics exceeded 1 MiB.");
                child.kill("SIGKILL");
            }
        });
        child.on("error", (error) => {
            failure = error;
        });
        child.on("close", (code) => {
            clearTimeout(timer);
            progress.close();
            if (failure) {
                reject(failure);
            } else if (code !== 0 || completed !== count) {
                reject(
                    new BuildError(
                        "BUILD_FAILED",
                        `Prerender failed (${completed}/${count}): ${diagnostics.trim()}`,
                    ),
                );
            } else {
                resolve();
            }
        });
    });
}
