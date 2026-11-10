import { OAUTH_PATH } from "@destack/account/address";
import { AUTHORIZATION_PATH } from "@destack/account/authentication";
import type { Environment } from "@destack/environment";
import { Failure } from "@destack/error";
import { type Accessor, createSignal } from "@destack/view";

/** Call an endpoint below the issuer on the page's own origin, with the browser's session. */
export async function callIssuer(
    path: string,
    environment: Pick<Environment, "network">,
    body?: object,
    method: "GET" | "POST" | "DELETE" = body === undefined ? "GET" : "POST",
): Promise<unknown> {
    // send JSON with the session, as the issuer's own origin
    const response = await environment.network.fetch(
        new Request(`${OAUTH_PATH}${path}`, {
            method,
            headers: { "content-type": "application/json", accept: "application/json" },
            credentials: "same-origin",
            ...(body === undefined ? {} : { body: JSON.stringify(body) }),
        }),
    );

    // raise a refusal as the call error the issuer answered
    if (!response.ok) {
        throw Failure.error(await Failure.read(response));
    }
    const answer: unknown = response.status === 204 ? null : await response.json();

    return answer;
}

/** Write where a sign-in continues: the stored authorization request, else its page. */
export function continuation(query: URLSearchParams): string {
    // continue a stored authorization request at the authorization endpoint
    const clientId = query.get("client_id");
    const requestUri = query.get("request_uri");
    if (clientId !== null && requestUri !== null) {
        const request = new URLSearchParams({ client_id: clientId, request_uri: requestUri });

        return `${OAUTH_PATH}${AUTHORIZATION_PATH}?${request}`;
    }

    // return to the page that sent the person, else Home
    return query.get("callbackURL") ?? "/";
}

/** A view's step at the issuer: whether one is on its way, and the issuer's refusal of the last. */
export interface IssuerStep {
    /** The refusal of the last step, absent for none. */
    readonly error: Accessor<string | undefined>;
    /** Whether a step is on its way. */
    readonly isPending: Accessor<boolean>;
    /** Run a step, showing the issuer's refusal and reporting any other failure. */
    readonly run: (call: () => Promise<void>) => void;
}

/** Create a view's step at the issuer, showing a refusal the page's address carries first. */
export function createIssuerStep(refusal?: string): IssuerStep {
    const [error, setError] = createSignal(refusal);
    const [isPending, setPending] = createSignal(false);

    return {
        error,
        isPending,
        run: (call) => {
            // clear the last refusal while the step is on its way
            setPending(true);
            setError(undefined);
            call()
                .catch((refused: unknown) => {
                    if (Failure.isDeclared(refused)) {
                        setError(Failure.of(refused).message);
                    } else {
                        globalThis.reportError(refused);
                    }
                })
                .finally(() => setPending(false));
        },
    };
}
