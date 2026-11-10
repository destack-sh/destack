import { UNIVERSE_PAGES } from "@destack/account/address";
import { argument, t } from "@destack/locale";
import type { schema } from "@destack/schema";
import {
    AUTHORIZATION_REQUEST_PATH,
    AuthorizationRequestOutcome,
    AuthorizationRequestReview,
    type Decision,
} from "@destack/account/authentication";
import { ConsentPrompt } from "@destack/ui/consent";
import { createSignal, onSettled, Show, useEnvironment, useLocale } from "@destack/view";
import { GrantPermissions } from "./grant.tsx";
import { CardPage } from "./page.tsx";
import { callIssuer, createIssuerStep } from "./issuer.ts";

/** A reviewed authorization request. */
type Review = schema.Infer<typeof AuthorizationRequestReview>;

/** Ask the person to allow or refuse an application's stored request, signing in first if due. */
export default function ConsentPage() {
    // read the stored request the page decides, refusing a page opened without one
    const environment = useEnvironment();
    const locale = useLocale();
    const page = new URL(environment.navigation.currentEntry.url);
    const requestUri = page.searchParams.get("request_uri");
    const [request, setRequest] = createSignal<Review>();
    const { error, isPending, run } = createIssuerStep(
        requestUri === null
            ? locale.render(t`Open this page from an application's sign-in.`)
            : undefined,
    );

    // read the request, sending a person who must sign in again to the sign-in view
    onSettled(() => {
        if (requestUri !== null) {
            run(async () => {
                const query = new URLSearchParams({ request_uri: requestUri });
                const read = AuthorizationRequestReview.parse(
                    await callIssuer(`${AUTHORIZATION_REQUEST_PATH}?${query}`, environment),
                );
                if (read.isLoginDue) {
                    environment.navigation.assign(`${UNIVERSE_PAGES.signIn}${page.search}`);
                } else {
                    setRequest(read);
                }
            });
        }
    });

    // title the page by the application asking
    const title = () => {
        const asked = request();

        return asked === undefined
            ? locale.render(t`Allow an application`)
            : locale.render(t`${argument("app")} wants access to your Destack account`, {
                  app: asked.client.name,
              });
    };
    const description = () => {
        const grant = request()?.grant ?? null;

        return grant === null
            ? undefined
            : locale.render(t`${argument("package")} in ${argument("space")}`, {
                  package: grant.package,
                  space: grant.space,
              });
    };

    // decide the request and follow the issuer back to the application
    const decide = (decision: Decision) =>
        run(async () => {
            const body = { requestUri, decision };
            const { location } = AuthorizationRequestOutcome.parse(
                await callIssuer(AUTHORIZATION_REQUEST_PATH, environment, body),
            );
            environment.navigation.assign(location);
        });

    // offer an answer for this sign-in alone only for a first grant
    const allowOnce = (asked: Review) => {
        const isFirst =
            asked.grant !== null && asked.grant.permissions.every((each) => !each.isGranted);

        return isFirst ? () => decide("approve-once") : undefined;
    };

    return (
        <CardPage title={title()} description={description()} error={error()}>
            <Show when={request()}>
                {(asked) => (
                    <ConsentPrompt
                        isPending={isPending()}
                        onAllow={() => decide("approve")}
                        onAllowOnce={allowOnce(asked())}
                        onDeny={() => decide("deny")}
                    >
                        <GrantPermissions grant={asked().grant} scopes={asked().scopes} />
                    </ConsentPrompt>
                )}
            </Show>
        </CardPage>
    );
}
