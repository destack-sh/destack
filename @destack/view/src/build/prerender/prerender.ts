import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import type { Environment, Subprocess } from "@destack/environment";
import { PackagePath } from "@destack/package/file";
import { present, schema } from "@destack/schema";
import { ViewError } from "../../error/error.ts";
import type { PageAssets, StaticRoutes } from "../../static/page.ts";

/** The longest a request's response may take, in milliseconds. */
const RESPONSE_TIMEOUT = 10_000;

/** The most renderer diagnostics kept: 1 MiB, far more than any failure prints. */
const MAX_DIAGNOSTICS = 1024 * 1024;

/** The file in a render directory holding its plan. */
export const RENDER_PLAN_FILE = "plan.json";

/** The file in a render directory listing the requests the renderer answered. */
export const RENDERED_FILE = "rendered.json";

/** What a renderer renders: the server module's URL, the site's origin and the pages' assets. */
export const RenderPlan = schema.object({
    /** The file URL of the server module answering the requests. */
    module: schema.string(),
    /** The site's public origin. */
    origin: schema.url(),
    /** The assets each page links. */
    assets: schema.object({
        /** The module each page loads. */
        script: schema.string(),
        /** The stylesheets each page links, in order. */
        stylesheets: schema.array(schema.string()).readonly(),
    }),
});
/** What a renderer renders. */
export type RenderPlan = schema.Infer<typeof RenderPlan>;

/** One request a build renders: its URL, its file and the response it requires. */
export const RenderRequest = schema.object({
    /** The rendered URL. */
    url: schema.string(),
    /** The file it writes below the view's pages directory. */
    path: PackagePath,
    /** What it answers: a page of HTML, the not-found page, or an endpoint's file. */
    kind: schema.enum(["page", "missing", "endpoint"]),
});
/** One request a build renders. */
export type RenderRequest = schema.Infer<typeof RenderRequest>;

/** The status and headers the renderer writes beside each body. */
const RenderedResponse = schema.object({
    /** The response status. */
    status: schema.number(),
    /** The response headers by lowercase name. */
    headers: schema.record(schema.string(), schema.string()),
});
/** The status and headers the renderer writes beside each body. */
type RenderedResponse = schema.Infer<typeof RenderedResponse>;

/** Render a static view's routes with its server module, its files by path. */
export async function prerender(
    module: string,
    origin: string,
    assets: PageAssets,
    environment: Environment<"files" | "process">,
): Promise<Map<string, Uint8Array<ArrayBuffer>>> {
    // run the renderer in a directory of its own
    const { files } = environment;
    const rendered = new Map<string, Uint8Array<ArrayBuffer>>();
    const directory = await files.makeTemporaryDirectory("destack-render-");
    try {
        // write the plan and render every route
        const plan: RenderPlan = { module: pathToFileURL(module).href, origin, assets };
        await files.writeTextFile(join(directory, RENDER_PLAN_FILE), JSON.stringify(plan));
        await runRenderer(directory, RESPONSE_TIMEOUT, environment);

        // read each answered request's file, requiring the response its kind needs
        const text = await files.readTextFile(join(directory, RENDERED_FILE));
        const requests = schema.array(RenderRequest).parse(JSON.parse(text));
        for (const [index, request] of requests.entries()) {
            rendered.set(request.path, await readAnswer(directory, index, request, environment));
        }
    } finally {
        await files.remove(directory, { recursive: true });
    }

    return rendered;
}

/** Plan each route's request and file, refusing uncanonical or duplicate paths. */
export function planRequests(origin: string, routes: StaticRoutes): RenderRequest[] {
    // list the pages, the not-found page and the endpoints
    const site = new URL(origin);
    const planned = [
        ...routes.paths.map((path) => ({ path, kind: "page" as const })),
        ...(routes.notFound === undefined
            ? []
            : [{ path: routes.notFound, kind: "missing" as const }]),
        ...routes.endpoints.map((path) => ({ path, kind: "endpoint" as const })),
    ];

    // require a canonical path on the site's origin for each
    const requests = planned.map(({ path, kind }) => {
        const url = new URL(path, site);
        if (url.origin !== site.origin || url.pathname !== path || url.search || url.hash) {
            throw ViewError.of(
                "ROUTE_INVALID",
                `path '${path}' is invalid: it must be a canonical URL path`,
            );
        }

        return { url: url.href, path: fileOf(path, kind), kind };
    });

    // refuse two routes writing one file
    if (new Set(requests.map((request) => request.path)).size !== requests.length) {
        throw ViewError.of(
            "PAGE_INVALID",
            "prerender route set is invalid: the routes produce duplicate files",
        );
    }

    return requests;
}

/** Name a route's file: 404.html, a page's index.html, or an endpoint's own path. */
function fileOf(route: string, kind: RenderRequest["kind"]): string {
    // name the not-found page
    if (kind === "missing") {
        return "404.html";
    }
    // name an endpoint's file by its path
    else if (kind === "endpoint") {
        return PackagePath.parse(decodeURIComponent(route).replace(/^\//u, ""));
    }
    // name a page's index
    else {
        const directory = decodeURIComponent(route).replace(/^\/|\/$/gu, "");

        return PackagePath.parse(`${directory}/index.html`.replace(/^\//u, ""));
    }
}

/** Read an answered request's body, refusing an unexpected or personalized answer. */
async function readAnswer(
    directory: string,
    index: number,
    request: RenderRequest,
    environment: Environment<"files">,
): Promise<Uint8Array<ArrayBuffer>> {
    // read the response the renderer wrote beside the body
    const { response, body } = await readRendered(directory, index, request, environment);

    // require the status its kind needs, and HTML for a page
    const status = request.kind === "missing" ? 404 : 200;
    const contentType = response.headers["content-type"];
    const isHtml = contentType?.startsWith("text/html") === true;
    if (response.status !== status || (request.kind !== "endpoint" && !isHtml)) {
        throw ViewError.of(
            "RESPONSE_INVALID",
            `response for '${request.url}' is invalid: it must be '${request.kind === "endpoint" ? "a file" : "an HTML page"}', got status ${response.status}`,
        );
    }

    // refuse a response varying by request or marked private
    const cacheControl = response.headers["cache-control"];
    if (
        response.headers["set-cookie"] !== undefined ||
        (cacheControl !== undefined && /private|no-store/iu.test(cacheControl)) ||
        response.headers["vary"] !== undefined
    ) {
        throw ViewError.of(
            "RESPONSE_INVALID",
            `response for '${request.url}' is invalid: a personalized response cannot prerender`,
        );
    }

    return body;
}

/** Read the status, headers and body the renderer wrote for a request. */
async function readRendered(
    directory: string,
    index: number,
    request: RenderRequest,
    environment: Environment<"files">,
): Promise<{ response: RenderedResponse; body: Uint8Array<ArrayBuffer> }> {
    try {
        // read the metadata, then the body
        const { files } = environment;
        const metadata = await files.readTextFile(join(directory, `${index}.json`));
        const response = RenderedResponse.parse(JSON.parse(metadata));
        const body = await files.readFile(join(directory, `${index}.body`));

        return { response, body };
    } catch (cause) {
        throw ViewError.of(
            "PRERENDER_INVALID",
            `prerendered '${request.url}' is invalid: it is unreadable`,
            {
                cause,
            },
        );
    }
}

/** Render a directory's plan in the renderer, with a deadline between completed responses. */
async function runRenderer(
    directory: string,
    timeout: number,
    environment: Environment<"process">,
): Promise<void> {
    // start the renderer without the host's environment, keeping the user's path and home
    const { process } = environment;
    const renderer = fileURLToPath(import.meta.resolve("@destack/view/prerender"));
    const { PATH: path, HOME: home } = process.variables;
    const child = process.spawn([process.executable, "run", "--no-env-file", renderer, directory], {
        workingDirectory: directory,
        variables: {
            ...(path === undefined ? {} : { PATH: path }),
            ...(home === undefined ? {} : { HOME: home }),
            NO_COLOR: "1",
        },
        stdout: "pipe",
        stderr: "pipe",
    });
    await supervise(child, timeout, environment);
}

/** Wait for a renderer to plan and answer every request, each within the timeout. */
async function supervise(
    child: Subprocess,
    timeout: number,
    environment: Pick<Environment, "clock">,
): Promise<void> {
    // stop the renderer once its plan or a response runs past the timeout
    let completed = 0;
    let count: number | undefined;
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
                    ViewError.of(
                        "PRERENDER_UNREACHABLE",
                        `could not prerender the next response within ${timeout} ms after ${completed} items`,
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

    // read the planned count, count the responses and bound the diagnostics
    const progress = readLines(child.stdout, (line) => {
        const planned = /^\{"planned":(\d+)\}$/u.exec(line)?.[1];
        if (count === undefined && planned !== undefined) {
            count = Number(planned);
            arm();
        } else if (
            count !== undefined &&
            completed < count &&
            line === JSON.stringify({ rendered: completed })
        ) {
            completed++;
            arm();
        }
    });
    const diagnostics = readDiagnostics(child.stderr, () =>
        stop(
            ViewError.of(
                "DIAGNOSTICS_INVALID",
                "prerender diagnostic output is invalid: it exceeded 1 MiB",
            ),
        ),
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
    count: number | undefined,
    diagnostics: string,
): Error | undefined {
    return code !== 0 || completed !== count
        ? ViewError.of(
              "PRERENDER_INVALID",
              `prerender is invalid: it finished ${completed}/${count ?? "?"} with diagnostics:\n${diagnostics.trim()}`,
          )
        : undefined;
}
