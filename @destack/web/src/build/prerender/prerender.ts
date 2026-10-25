import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import type { Environment, Subprocess } from "@destack/environment";
import { PackagePath } from "@destack/package/file";
import { present, schema } from "@destack/schema";
import { WebError } from "../../error/error.ts";

/** The longest a request's response may take by default, in milliseconds. */
const RESPONSE_TIMEOUT = 10_000;

/** The most renderer diagnostics kept: 1 MiB, far more than any failure prints. */
const MAX_DIAGNOSTICS = 1024 * 1024;

/** Requests rendered to static HTML during a build, at the site's origin. */
export const PrerenderOptions = schema.object({
    /** URL paths to render, including a leading slash. */
    routes: schema.array(schema.string()).readonly(),
    /** Route returning 404 to publish as 404.html. */
    notFound: schema.string().exactOptional(),
    /** Maximum time per request, including its body, in milliseconds. */
    timeout: schema.number().exactOptional(),
});
/** Requests rendered to static HTML during a build. */
export type PrerenderOptions = schema.Infer<typeof PrerenderOptions>;

/** The file in a render directory holding its plan. */
export const RENDER_PLAN_FILE = "plan.json";

/** What a renderer renders: the server module's URL and the page URLs, oldest first. */
export const RenderPlan = schema.object({
    /** The file URL of the server module handling the requests. */
    module: schema.string(),
    /** The URLs to render, by page index. */
    urls: schema.array(schema.string()),
});
/** What a renderer renders. */
export type RenderPlan = schema.Infer<typeof RenderPlan>;

/** The status and headers the renderer writes beside each page body. */
const RenderedResponse = schema.object({
    /** The response status. */
    status: schema.number(),
    /** The response headers by lowercase name. */
    headers: schema.record(schema.string(), schema.string()),
});
/** The status and headers the renderer writes beside each page body. */
type RenderedResponse = schema.Infer<typeof RenderedResponse>;

/** One page a build renders: its URL, its file and the status it requires. */
interface PageRequest {
    /** The rendered URL. */
    readonly url: URL;
    /** The page's file in the browser output. */
    readonly path: string;
    /** The status the response requires. */
    readonly status: number;
}

/** Render explicit public routes at a site's origin into a browser output directory. */
export async function prerender(
    module: string,
    site: string,
    options: PrerenderOptions,
    environment: Environment<"files" | "process">,
): Promise<Map<string, Uint8Array<ArrayBuffer>>> {
    // reject ambiguous URLs before executing application code
    const requests = planRequests(site, options);

    // reuse the renderer across requests
    const { files } = environment;
    const pages = new Map<string, Uint8Array<ArrayBuffer>>();
    const directory = await files.makeTemporaryDirectory("destack-render-");
    try {
        await writePlan(directory, module, requests, environment);
        const timeout = options.timeout ?? RESPONSE_TIMEOUT;
        await runRenderer(directory, timeout, requests.length, environment);
        for (const [index, request] of requests.entries()) {
            pages.set(request.path, await readPage(directory, index, request, environment));
        }
    } finally {
        await files.remove(directory, { recursive: true });
    }

    return pages;
}

/** Plan each route's request and page, refusing uncanonical paths and duplicate files. */
function planRequests(site: string, options: PrerenderOptions): PageRequest[] {
    // require a canonical path on the site's origin for each route
    const origin = new URL(site);
    const routes = [
        ...options.routes,
        ...(options.notFound === undefined ? [] : [options.notFound]),
    ];
    const requests = routes.map((route) => {
        // resolve the route against the origin
        const url = new URL(route, origin);
        if (url.origin !== origin.origin || url.pathname !== route || url.search || url.hash) {
            throw WebError.of("ROUTE_NOT_CANONICAL", `expected a canonical URL path: ${route}`);
        }
        const isNotFound = route === options.notFound;

        return { url, path: pagePath(route, isNotFound), status: isNotFound ? 404 : 200 };
    });

    // refuse two routes writing one file
    if (new Set(requests.map((request) => request.path)).size !== requests.length) {
        throw WebError.of("PAGE_DUPLICATE", "prerender routes produce duplicate files");
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

/** Write the plan the renderer reads: the server module and each request's URL. */
async function writePlan(
    directory: string,
    module: string,
    requests: readonly PageRequest[],
    environment: Environment<"files">,
): Promise<void> {
    const plan: RenderPlan = {
        module: pathToFileURL(module).href,
        urls: requests.map((request) => request.url.href),
    };
    await environment.files.writeTextFile(join(directory, RENDER_PLAN_FILE), JSON.stringify(plan));
}

/** Read a rendered page, refusing a response of another status, not HTML, or personalized. */
async function readPage(
    directory: string,
    index: number,
    request: PageRequest,
    environment: Environment<"files">,
): Promise<Uint8Array<ArrayBuffer>> {
    // read the response the renderer wrote beside the page
    const { response, body } = await readRendered(directory, index, request, environment);

    // require the expected status and an HTML body
    const contentType = response.headers["content-type"];
    if (response.status !== request.status || contentType?.startsWith("text/html") !== true) {
        throw WebError.of(
            "RESPONSE_UNEXPECTED",
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
        throw WebError.of(
            "PERSONALIZED_RESPONSE",
            `cannot prerender a personalized response: ${request.url.pathname}`,
        );
    }

    return body;
}

/** Read the status, headers and body the renderer wrote for a request. */
async function readRendered(
    directory: string,
    index: number,
    request: PageRequest,
    environment: Environment<"files">,
): Promise<{ response: RenderedResponse; body: Uint8Array<ArrayBuffer> }> {
    try {
        // read the metadata, then the page body
        const { files } = environment;
        const metadata = await files.readTextFile(join(directory, `${index}.json`));
        const response = RenderedResponse.parse(JSON.parse(metadata));
        const body = await files.readFile(join(directory, `${index}.html`));

        return { response, body };
    } catch (cause) {
        throw WebError.of(
            "PRERENDER_UNREADABLE",
            `the prerendered page ${request.url.pathname} is unreadable`,
            { cause },
        );
    }
}

/** Render a directory's plan in the renderer, with a deadline between completed responses. */
async function runRenderer(
    directory: string,
    timeout: number,
    count: number,
    environment: Environment<"process">,
): Promise<void> {
    // start the renderer without the host's environment
    const { process } = environment;
    const renderer = fileURLToPath(import.meta.resolve("@destack/web/prerender"));
    const path = process.variables["PATH"];
    const child = process.spawn([process.executable, "run", "--no-env-file", renderer, directory], {
        workingDirectory: directory,
        variables: {
            ...(path === undefined ? {} : { PATH: path }),
            NO_COLOR: "1",
        },
        stdout: "pipe",
        stderr: "pipe",
    });
    await supervise(child, timeout, count, environment);
}

/** Wait for a renderer to report every response, each within the timeout of the last. */
async function supervise(
    child: Subprocess,
    timeout: number,
    count: number,
    environment: Pick<Environment, "clock">,
): Promise<void> {
    // stop the renderer once a response runs past the timeout
    let completed = 0;
    let failure: Error | undefined;
    const stop = (error: Error) => {
        failure ??= error;
        child.kill("SIGKILL");
    };
    let deadline = new AbortController();
    const arm = () => {
        // restart the deadline
        deadline.abort();
        deadline = new AbortController();
        const step = deadline.signal;
        void environment.clock.sleep(timeout, step).then(
            () =>
                stop(
                    WebError.of(
                        "PRERENDER_TIMEOUT",
                        `prerender response ${completed} exceeded ${timeout} ms`,
                    ),
                ),
            (error: unknown) => {
                if (!step.aborted) {
                    throw error;
                }
            },
        );
    };
    arm();

    // count the responses and bound the diagnostics
    const progress = readLines(child.stdout, (line) => {
        if (completed < count && line === JSON.stringify({ rendered: completed })) {
            completed++;
            arm();
        }
    });
    const diagnostics = readDiagnostics(child.stderr, () =>
        stop(WebError.of("DIAGNOSTICS_TOO_LONG", "prerender diagnostics exceeded 1 MiB")),
    );

    // wait for the renderer to end, stopping the deadline however the wait ends
    let ended: readonly [number, void, string];
    try {
        ended = await Promise.all([child.exited, progress, diagnostics]);
    } finally {
        deadline.abort();
    }

    // report what stopped the renderer, an unclean exit or a missing response
    const [code, , text] = ended;
    const error = failure ?? exitFailure(code, completed, count, text);
    if (error !== undefined) {
        throw error;
    }
}

/** Call a listener with each line a stream's text carries, until it ends. */
async function readLines(
    stream: ReadableStream<Uint8Array>,
    line: (text: string) => void,
): Promise<void> {
    // split the text at line ends
    const decoder = new TextDecoder();
    let pending = "";
    for await (const chunk of stream) {
        const lines = `${pending}${decoder.decode(chunk, { stream: true })}`.split("\n");
        pending = present(lines.pop(), "a split line");
        for (const each of lines) {
            line(each);
        }
    }
}

/** Read a stream's text up to the diagnostics bound, calling a listener once it runs over. */
async function readDiagnostics(
    stream: ReadableStream<Uint8Array>,
    overflow: () => void,
): Promise<string> {
    // keep the text until it runs past the bound
    const decoder = new TextDecoder();
    let text = "";
    for await (const chunk of stream) {
        // stop keeping text past the bound
        if (text.length > MAX_DIAGNOSTICS) {
            continue;
        }
        text += decoder.decode(chunk, { stream: true });
        if (text.length > MAX_DIAGNOSTICS) {
            overflow();
        }
    }

    return text;
}

/** Read the failure of a renderer that exited uncleanly or early, absent otherwise. */
function exitFailure(
    code: number,
    completed: number,
    count: number,
    diagnostics: string,
): Error | undefined {
    return code !== 0 || completed !== count
        ? WebError.of(
              "PRERENDER_EXITED",
              `prerender failed (${completed}/${count}): ${diagnostics.trim()}`,
          )
        : undefined;
}
