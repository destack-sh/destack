import { Failure } from "@destack/error";

/** The states the platform answers a page load in when no view can. */
export type ViewState = "starting" | "not-found" | "error";

/** The status of an app still starting, which a browser loads again after its Retry-After. */
const STARTING_STATUS = 503;

/** The status of a missing app or view. */
const NOT_FOUND_STATUS = 404;

/** The headers a state's page keeps from its refusal, such as a cleared session cookie. */
const KEPT_HEADERS = ["retry-after", "set-cookie", "cache-control"];

/** The title of each state's page. */
const TITLES: Readonly<Record<ViewState, string>> = {
    starting: "Starting…",
    "not-found": "Not found",
    error: "Something went wrong",
};

/** The pages of the states the platform answers page loads in. */
export const ViewState = {
    /** Report whether a request loads a page, as browsers mark a document navigation. */
    isPageLoad(request: Request): boolean {
        return (
            request.headers.get("sec-fetch-mode") === "navigate" &&
            request.headers.get("sec-fetch-dest") === "document"
        );
    },

    /** Read the state of a refusal's status. */
    of(status: number): ViewState {
        // read an app starting, a missing one, and any other failure
        if (status === STARTING_STATUS) {
            return "starting";
        } else if (status === NOT_FOUND_STATUS) {
            return "not-found";
        } else {
            return "error";
        }
    },

    /** Answer the platform's refusal of a page load with its state's page. */
    async document(response: Response): Promise<Response> {
        // read the refusal's message, and how long a starting app asks to wait
        const state = ViewState.of(response.status);
        const failure = await Failure.read(response);
        const retryAfter = response.headers.get("retry-after");
        const refresh =
            state === "starting" && retryAfter !== null
                ? `<meta http-equiv="refresh" content="${retryAfter}">\n`
                : "";

        // keep the refusal's status and the headers its page needs
        const headers = new Headers({ "content-type": "text/html; charset=utf-8" });
        for (const name of KEPT_HEADERS) {
            const values =
                name === "set-cookie"
                    ? response.headers.getSetCookie()
                    : [response.headers.get(name)];
            for (const value of values) {
                if (value !== null) {
                    headers.append(name, value);
                }
            }
        }
        const html = `<!doctype html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="light dark">
${refresh}<title>${escape(TITLES[state])}</title>
<style>
body { margin: 0; min-height: 100vh; display: grid; place-items: center; font: 15px/1.5 system-ui, sans-serif; background: Canvas; color: CanvasText; }
main { max-width: 32rem; padding: 2rem; text-align: center; }
h1 { margin: 0 0 0.5rem; font-size: 1.25rem; font-weight: 600; }
p { margin: 0; opacity: 0.7; }
</style>
</head>
<body>
<main data-state="${state}">
<h1>${escape(TITLES[state])}</h1>
<p>${escape(failure.message)}</p>
</main>
</body>
</html>
`;

        return new Response(html, { status: response.status, headers });
    },
};

/** Escape text for HTML. */
function escape(text: string): string {
    return text
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll('"', "&quot;");
}
