import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { pathToFileURL } from "node:url";
import { PackagePath } from "@destack/package/file";
import { schema } from "@destack/schema";
import { type ChildProcessByStdio, spawn } from "node:child_process";
import type { Readable } from "node:stream";
import { createInterface } from "node:readline";

/** The longest a request's response may take by default, in milliseconds. */
const RESPONSE_TIMEOUT = 10_000;

/** The most renderer diagnostics kept: 1 MiB, far more than any failure prints. */
const MAX_DIAGNOSTICS = 1024 * 1024;

/** Requests rendered to static HTML during a build. */
export const PrerenderOptions = schema.object({
    /** Absolute origin used for build-time requests. */
    origin: schema.string(),
    /** URL paths to render, including a leading slash. */
    routes: schema.array(schema.string()).readonly(),
    /** Route returning 404 to publish as 404.html. */
    notFound: schema.string().exactOptional(),
    /** Maximum time per request, including its body, in milliseconds. */
    timeout: schema.number().exactOptional(),
});
/** Requests rendered to static HTML during a build. */
export type PrerenderOptions = schema.Infer<typeof PrerenderOptions>;

/** The status and headers the renderer writes beside each page body. */
const RenderedResponse = schema.object({
    /** The response status. */
    status: schema.number(),
    /** The response headers by lowercase name. */
    headers: schema.record(schema.string(), schema.string()),
});

/** One page a build renders: its URL, its file and the status it requires. */
interface PageRequest {
    /** The rendered URL. */
    readonly url: URL;
    /** The page's file in the browser output. */
    readonly path: string;
    /** The status the response requires. */
    readonly status: number;
}

/** Render explicit public routes into a browser output directory. */
export async function prerender(
    module: string,
    options: PrerenderOptions,
): Promise<Map<string, Uint8Array<ArrayBuffer>>> {
    // reject ambiguous URLs before executing application code
    const requests = planRequests(options);

    // reuse the renderer across requests
    const files = new Map<string, Uint8Array<ArrayBuffer>>();
    const directory = await mkdtemp(join(tmpdir(), "destack-render-"));
    try {
        const program = await writeProgram(directory, module, requests);
        await runProgram(program, options.timeout ?? RESPONSE_TIMEOUT, requests.length);
        for (const [index, request] of requests.entries()) {
            files.set(request.path, await readPage(directory, index, request));
        }
    } finally {
        await rm(directory, { recursive: true });
    }

    return files;
}

/** Plan each route's request and page, refusing a non-HTTP origin, uncanonical paths and duplicate files. */
function planRequests(options: PrerenderOptions): PageRequest[] {
    // require an HTTP origin
    const origin = new URL(options.origin);
    if (origin.origin !== options.origin || !["http:", "https:"].includes(origin.protocol)) {
        throw new Error(`expected an HTTP origin: ${options.origin}`);
    }

    // require a canonical path on the same origin for each route
    const routes = [
        ...options.routes,
        ...(options.notFound === undefined ? [] : [options.notFound]),
    ];
    const requests = routes.map((route) => {
        // resolve the route against the origin
        const url = new URL(route, origin);
        if (url.origin !== origin.origin || url.pathname !== route || url.search || url.hash) {
            throw new Error(`expected a canonical URL path: ${route}`);
        }
        const isNotFound = route === options.notFound;

        return { url, path: pagePath(route, isNotFound), status: isNotFound ? 404 : 200 };
    });

    // refuse two routes writing one file
    if (new Set(requests.map((request) => request.path)).size !== requests.length) {
        throw new Error("prerender routes produce duplicate files");
    }

    return requests;
}

/** Name a route's page file: 404.html for the not-found route, else the route's index.html. */
function pagePath(route: string, isNotFound: boolean): string {
    // name the not-found page
    if (isNotFound) {
        return "404.html";
    }

    // name the route's index page
    const directory = decodeURIComponent(route).replace(/^\/|\/$/gu, "");

    return PackagePath.parse(`${directory}/index.html`.replace(/^\//u, ""));
}

/** Write the program rendering each request with the server module, returning its path. */
async function writeProgram(
    directory: string,
    module: string,
    requests: readonly PageRequest[],
): Promise<string> {
    // import the server module and the renderer to render every URL
    const program = join(directory, "render.js");
    const renderer = new URL("./render.ts", import.meta.url).href;
    const urls = requests.map((request) => request.url.href);
    await writeFile(
        program,
        [
            `import * as server from ${JSON.stringify(pathToFileURL(module).href)};`,
            `import { renderPages } from ${JSON.stringify(renderer)};`,
            `await renderPages(server.handleRequest, ${JSON.stringify(urls)}, ${JSON.stringify(directory)});`,
        ].join("\n"),
    );

    return program;
}

/** Read a rendered page, refusing a response of another status, not HTML, or personalized. */
async function readPage(
    directory: string,
    index: number,
    request: PageRequest,
): Promise<Uint8Array<ArrayBuffer>> {
    try {
        // require the expected status and an HTML body
        const metadata = join(directory, `${index}.json`);
        const response = RenderedResponse.parse(JSON.parse(await readFile(metadata, "utf8")));
        const contentType = response.headers["content-type"];
        if (response.status !== request.status || contentType?.startsWith("text/html") !== true) {
            throw new Error(
                `expected an HTML response: ${request.url.pathname} (${response.status})`,
            );
        }

        // refuse a response varying by request or marked private
        const cacheControl = response.headers["cache-control"];
        if (
            response.headers["set-cookie"] !== undefined ||
            (cacheControl !== undefined && /private|no-store/iu.test(cacheControl)) ||
            response.headers["vary"] !== undefined
        ) {
            throw new Error(`cannot prerender a personalized response: ${request.url.pathname}`);
        }

        return new Uint8Array(await readFile(join(directory, `${index}.html`)));
    } catch (cause) {
        throw new Error(`prerender failed: ${request.url.pathname}`, { cause });
    }
}

/** Render trusted source with a deadline between completed responses. */
async function runProgram(program: string, timeout: number, count: number): Promise<void> {
    // start the renderer without the host's environment
    const child = spawn(process.execPath, ["run", "--no-env-file", program], {
        cwd: dirname(program),
        env: { PATH: process.env["PATH"], NO_COLOR: "1" },
        stdio: ["ignore", "pipe", "pipe"],
    });
    await supervise(child, timeout, count);
}

/** Wait for a renderer to report every response, killing it past a response's deadline or the diagnostics bound. */
function supervise(
    child: ChildProcessByStdio<null, Readable, Readable>,
    timeout: number,
    count: number,
): Promise<void> {
    return new Promise<void>((resolve, reject) => {
        // stop the renderer when a response exceeds the timeout
        let completed = 0;
        let diagnostics = "";
        let failure: Error | undefined;
        const expire = () => {
            failure = new Error(`prerender response ${completed} exceeded ${timeout} ms`);
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

        // bound the diagnostics kept from the renderer
        child.stderr.setEncoding("utf8");
        child.stderr.on("data", (chunk: string) => {
            diagnostics += chunk;
            if (diagnostics.length > MAX_DIAGNOSTICS) {
                failure = new Error("prerender diagnostics exceeded 1 MiB");
                child.kill("SIGKILL");
            }
        });
        child.on("error", (error) => {
            failure = error;
        });

        // settle once the renderer exits
        child.on("close", (code) => {
            // stop the deadline and the progress reader
            clearTimeout(timer);
            progress.close();

            // report the failure that stopped the renderer, else an unclean exit or missing responses
            const error = failure ?? exitFailure(code, completed, count, diagnostics);
            if (error === undefined) {
                resolve();
            } else {
                reject(error);
            }
        });
    });
}

/** Read the failure of a renderer that exited uncleanly or before every response, absent otherwise. */
function exitFailure(
    code: number | null,
    completed: number,
    count: number,
    diagnostics: string,
): Error | undefined {
    return code !== 0 || completed !== count
        ? new Error(`prerender failed (${completed}/${count}): ${diagnostics.trim()}`)
        : undefined;
}
