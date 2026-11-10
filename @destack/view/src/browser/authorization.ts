import { REQUEST_PATH } from "@destack/account/address";
import { PermissionReference, StepUp } from "@destack/access";
import type { BrowserEnvironment } from "@destack/environment/browser";
import { Failure } from "@destack/error";
import { schema } from "@destack/schema";

/** The name of the window a page authorizes its person's calls again in, and of its channel. */
export const AUTHORIZE_WINDOW = "destack-authorize";

/** The data of a refusal a grant would lift: the permission the call needs. */
const GrantRefusal = schema.looseObject({ permission: PermissionReference });

/** The browser a page reaches its person through: its network, channels and navigation. */
type AuthorizeEnvironment = Pick<BrowserEnvironment, "network" | "broadcast" | "navigation">;

/** A page's calls, each sent again once its person authorizes what its refusal asks. */
export class PageAuthorization {
    /** The page's origin with its base, which serves the request. */
    readonly #root: string;
    /** The browser the page reaches its person through. */
    readonly #environment: AuthorizeEnvironment;

    /** Authorize through a page's origin, in a browser. */
    constructor(root: string, environment: AuthorizeEnvironment) {
        this.#root = root;
        this.#environment = environment;
    }

    /** Write the address requesting what lifts a refusal from the page's person, if any. */
    static url(refusal: Failure, root: string, back: string): string | undefined {
        // request a permission a grant lacks by its package, type and name
        const query = new URLSearchParams({ return: back });
        const grant = GrantRefusal.safeParse(refusal.data);
        const stepUp = StepUp.schema.safeParse(refusal.data);
        if (refusal.reason === "GRANT_INSUFFICIENT" && grant.success) {
            const { packageId, type, name } = grant.data.permission;
            query.set("package", packageId);
            query.set("type", type);
            query.set("name", name);
        }
        // request a fresh sign-in at an assurance, within an age
        else if (refusal.reason === "AUTHENTICATION_INSUFFICIENT" && stepUp.success) {
            query.set("assurance", String(stepUp.data.assurance));
            if (stepUp.data.maxAge !== undefined) {
                query.set("max-age", String(stepUp.data.maxAge));
            }
        }
        // leave any other refusal to the call
        else {
            return undefined;
        }

        return `${root}${REQUEST_PATH}?${query.toString()}`;
    }

    /** Tell the page its authorization returned, reporting whether this is the window it opened. */
    static answer(environment: Pick<BrowserEnvironment, "broadcast">): boolean {
        // tell the page that opened this window, which closes it
        const isAuthorizing = window.name === AUTHORIZE_WINDOW && window.opener !== null;
        if (isAuthorizing) {
            const port = environment.broadcast.open(AUTHORIZE_WINDOW);
            port.send("answered");
            port.close();
        }

        return isAuthorizing;
    }

    /** Send a call, authorizing what its refusal asks in a window and sending it again. */
    async fetch(request: Request): Promise<Response> {
        // send the call, keeping a copy to send again
        const { network, navigation } = this.#environment;
        const again = request.clone();
        const response = await network.fetch(request);
        const refusal = response.ok ? undefined : await Failure.read(response.clone());
        const current = new URL(navigation.currentEntry.url);
        const back = `${current.pathname}${current.search}`;
        const url =
            refusal === undefined ? undefined : PageAuthorization.url(refusal, this.#root, back);
        if (url === undefined) {
            return response;
        }

        // request it in a window, else in the page itself, then send the call again once answered
        const requesting = window.open(url, AUTHORIZE_WINDOW, "popup");
        if (requesting === null) {
            navigation.assign(url);

            return response;
        }

        return (await this.#answered(requesting)) ? network.fetch(again) : response;
    }

    /** Wait until the authorization's window answers, closing it, or the person closes it. */
    #answered(requesting: Window): Promise<boolean> {
        // settle once, closing the window and leaving both listeners
        const port = this.#environment.broadcast.open(AUTHORIZE_WINDOW);
        const { promise, resolve } = Promise.withResolvers<boolean>();
        const settle = (isAnswered: boolean) => {
            // stop listening, close the window, and answer
            listening[Symbol.dispose]();
            port.close();
            window.removeEventListener("focus", closed);
            requesting.close();
            resolve(isAnswered);
        };

        // settle on the window's answer, or on the page's focus once the person closed it
        const closed = () => {
            if (requesting.closed) {
                settle(false);
            }
        };
        const listening = port.on(() => settle(true));
        window.addEventListener("focus", closed);

        return promise;
    }
}
