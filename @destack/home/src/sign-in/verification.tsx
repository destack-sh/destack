import { UNIVERSE_PAGES } from "@destack/account/address";
import {
    DEVICE_REQUEST_PATH,
    DeviceRequestOutcome,
    DeviceRequestReview,
} from "@destack/account/authentication";
import { Verification, type VerificationRequest } from "@destack/ui/verification";
import { argument, t } from "@destack/locale";
import { ConsentPrompt } from "@destack/ui/consent";
import { createSignal, onSettled, Show, useEnvironment, useLocale } from "@destack/view";
import { GrantPermissions } from "./grant.tsx";
import { CardPage } from "./page.tsx";
import { callIssuer, createIssuerStep } from "./issuer.ts";
import { Failure } from "@destack/error";

/** Sign a device in by the code it shows: look the code up, then approve or deny it. */
export default function VerificationPage() {
    // read the code a device's link carries, absent when the person types it
    const environment = useEnvironment();
    const locale = useLocale();
    const linked = new URL(environment.navigation.currentEntry.url).searchParams.get("user_code");
    const [request, setRequest] = createSignal<VerificationRequest>();
    const [scopes, setScopes] = createSignal<readonly string[]>([]);
    const step = createIssuerStep();

    // run a step for a code, sending a signed-out person to sign in and back first
    const run = (userCode: string, call: () => Promise<void>) =>
        step.run(() =>
            call().catch((refused: unknown) => {
                if (Failure.of(refused).code !== "UNAUTHORIZED") {
                    throw refused;
                }
                const back = `/verification?${new URLSearchParams({ user_code: userCode })}`;
                const signIn = new URLSearchParams({ callbackURL: back });
                environment.navigation.assign(`${UNIVERSE_PAGES.signIn}?${signIn}`);
            }),
        );

    // look a code up, showing who asks and where the sign-in stands
    const lookUp = (userCode: string) =>
        run(userCode, async () => {
            // read the sign-in the code found
            const query = new URLSearchParams({ user_code: userCode });
            const read = DeviceRequestReview.parse(
                await callIssuer(`${DEVICE_REQUEST_PATH}?${query}`, environment),
            );

            // show it with the scopes its device asks for
            setRequest({ userCode, client: read.client.name, status: read.status });
            setScopes(read.scopes);
        });

    // look the link's code up once the page settles
    onSettled(() => {
        if (linked !== null) {
            lookUp(linked);
        }
    });

    // decide the sign-in the code found
    const decide = (found: VerificationRequest, decision: "approve" | "deny") =>
        run(found.userCode, async () => {
            const body = { userCode: found.userCode, decision };
            const { status } = DeviceRequestOutcome.parse(
                await callIssuer(DEVICE_REQUEST_PATH, environment, body),
            );
            setRequest({ ...found, status });
        });

    // title the page by the device asking while its sign-in is pending
    const title = () => {
        const found = request();

        return found?.status === "pending"
            ? locale.render(t`${argument("app")} wants access to your Destack account`, {
                  app: found.client,
              })
            : locale.render(t`Sign in a device`);
    };
    const description = () => {
        const found = request();

        return found?.status === "pending"
            ? locale.render(t`The device shows the code ${argument("code")}.`, {
                  code: found.userCode,
              })
            : undefined;
    };

    return (
        <CardPage title={title()} description={description()} error={step.error()}>
            <Verification request={request()} isPending={step.isPending()} onSubmit={lookUp}>
                <Show when={request()}>
                    {(found) => (
                        <ConsentPrompt
                            isPending={step.isPending()}
                            onAllow={() => decide(found(), "approve")}
                            onDeny={() => decide(found(), "deny")}
                        >
                            <GrantPermissions grant={null} scopes={scopes()} />
                        </ConsentPrompt>
                    )}
                </Show>
            </Verification>
        </CardPage>
    );
}
