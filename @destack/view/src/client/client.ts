import { ClientConfiguration, ClientContext } from "@destack/service/client";
import { ServiceError } from "@destack/service/error";

/** Open the host-authenticated application context for a browser view. */
export async function openClient(
    browser: ViewBrowser,
    signal?: AbortSignal,
): Promise<ClientContext> {
    // consume the window credential before loading application connections
    const key = "@destack/view.connection";
    const token = browser.location.hash.slice(1) || browser.sessionStorage.getItem(key);
    if (!token) {
        throw new ServiceError("UNAUTHORIZED", { message: "view launch credential is missing" });
    }
    browser.sessionStorage.setItem(key, token);
    browser.history.replaceState(null, "", browser.location.pathname + browser.location.search);

    // authenticate configuration without allowing a redirect to expose credentials
    const response = await browser.fetch(new URL("/view", browser.location.origin), {
        headers: { authorization: `Bearer ${token}` },
        redirect: "error",
        cache: "no-store",
        signal,
    });
    if (!response.ok) {
        throw new ServiceError(
            response.status === 401
                ? "UNAUTHORIZED"
                : response.status === 403
                  ? "FORBIDDEN"
                  : "BAD_GATEWAY",
            {
                message: `view connection failed with status ${response.status}`,
            },
        );
    }
    const configuration = ClientConfiguration.parse(await response.json());

    // constrain browser credentials to the origin that authenticated this view
    for (const binding of configuration.services) {
        const url = new URL(binding.url, browser.location.origin);
        if (url.origin !== browser.location.origin || url.username || url.password || url.hash) {
            throw new ServiceError("FORBIDDEN", {
                message: "view service must use the application origin",
            });
        }
        binding.url = url.href;
    }

    return new ClientContext(configuration, {
        headers: { authorization: `Bearer ${token}` },
        fetch: (request, options) =>
            browser.fetch(request, {
                ...options,
                signal: signal ? AbortSignal.any([request.signal, signal]) : request.signal,
                redirect: "error",
            }),
    });
}

/** Browser APIs used to establish a window-scoped application connection. */
export interface ViewBrowser {
    /** Current navigation location. */
    readonly location: { hash: string; origin: string; pathname: string; search: string };
    /** Storage isolated to the current browser tab. */
    readonly sessionStorage: {
        getItem(key: string): string | null;
        setItem(key: string, value: string): void;
    };
    /** Navigation history from which the launch credential is removed. */
    readonly history: { replaceState(value: unknown, title: string, url: string): void };
    /** Browser HTTP transport. */
    readonly fetch: typeof fetch;
}
