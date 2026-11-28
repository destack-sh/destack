import { Failure } from "@destack/error";
import { type Message, SOURCE_LOCALE, Localization, t } from "@destack/locale";

/** The states a page shows when its view cannot draw. */
export type ViewState = "unavailable" | "not-found" | "forbidden" | "unauthorized" | "failed";

/** The headers a state's page keeps from its refusal, such as a cleared session cookie. */
const KEPT_HEADERS = ["retry-after", "set-cookie", "cache-control"];

/** How each state reads to the person: its title, and what happened or what to do. */
const STATES: Readonly<
    Record<ViewState, { readonly title: Message; readonly description: Message }>
> = {
    unavailable: {
        title: t.context("title")`Not available right now`,
        description: t`It should be back in a moment.`,
    },
    "not-found": {
        title: t.context("title")`Nothing here`,
        description: t`There is no page at this address, or it moved.`,
    },
    forbidden: {
        title: t.context("title")`You don't have access`,
        description: t`Ask whoever runs this space to share it with you.`,
    },
    unauthorized: {
        title: t.context("title")`Sign in to continue`,
        description: t`Your session ended, so sign in again.`,
    },
    failed: {
        title: t.context("title")`Something went wrong`,
        description: t`Try again in a moment.`,
    },
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

    /** Read the state a failure leaves a page in, by its call error code. */
    of(failure: Pick<Failure, "code">): ViewState {
        // read a page not served yet, a missing page, a denied or signed out person, and any other failure
        if (failure.code === "SERVICE_UNAVAILABLE") {
            return "unavailable";
        } else if (failure.code === "NOT_FOUND") {
            return "not-found";
        } else if (failure.code === "FORBIDDEN") {
            return "forbidden";
        } else if (failure.code === "UNAUTHORIZED") {
            return "unauthorized";
        } else {
            return "failed";
        }
    },

    /** Read how a state reads to the person: its title and its description. */
    describe(state: ViewState): { readonly title: Message; readonly description: Message } {
        return STATES[state];
    },

    /** Answer the platform's refusal of a page load with its state's page. */
    async document(response: Response): Promise<Response> {
        // read the refusal's state, and how long an unavailable page asks to wait
        const failure = await Failure.read(response);
        const state = ViewState.of(failure);
        const retryAfter = response.headers.get("retry-after");
        const refresh =
            state === "unavailable" && retryAfter !== null
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
        // write the state's title and description in the source language, as server pages do
        const localization = Localization.of(SOURCE_LOCALE, []);
        const { title, description } = STATES[state];
        const html = `<!doctype html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="light dark">
${refresh}<title>${escape(localization.render(title))}</title>
<style>
body { margin: 0; min-height: 100vh; display: grid; place-items: center; font: 15px/1.5 system-ui, sans-serif; background: Canvas; color: CanvasText; }
main { max-width: 32rem; padding: 2rem; text-align: center; }
h1 { margin: 0 0 0.5rem; font-size: 1.25rem; font-weight: 600; }
p { margin: 0; opacity: 0.7; }
</style>
</head>
<body>
<main data-state="${state}">
<h1>${escape(localization.render(title))}</h1>
<p>${escape(localization.render(description))}</p>
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
