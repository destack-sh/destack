import { UNIVERSE_PAGES } from "@destack/account/address";
import { schema } from "@destack/schema";
import { SignInCode, SignInEmail, SignInProvider, SignInSecondFactor } from "@destack/ui/sign-in";
import { t } from "@destack/locale";
import {
    createSignal,
    For,
    Match,
    onSettled,
    Switch,
    useEnvironment,
    useLocale,
} from "@destack/view";
import {
    type ProviderId,
    SIGN_IN_PROVIDERS_PATH,
    SignInProviders,
} from "@destack/account/authentication";
import { CardPage } from "./page.tsx";
import { callIssuer, continuation, createIssuerStep } from "./issuer.ts";

/** A provider's sign-in: the URL the browser goes to. */
const Redirection = schema.object({ url: schema.string() }).loose();

/** A first-factor sign-in: whether the person's second factor comes next. */
const SignedIn = schema.object({ twoFactorRedirect: schema.boolean().exactOptional() }).loose();

/** Sign a person in by a provider or a mailed code, then their second factor, and continue. */
export default function SignInPage() {
    // read the page's query: the stored request or page to continue, the step and a refusal to show
    const environment = useEnvironment();
    const locale = useLocale();
    const query = new URL(environment.navigation.currentEntry.url).searchParams;
    const next = continuation(query);
    const [step, setStep] = createSignal<"email" | "code" | "second-factor">(
        query.get("step") === "second-factor" ? "second-factor" : "email",
    );
    const [email, setEmail] = createSignal(query.get("email") ?? "");

    // run each step at the issuer, showing its refusal
    const { error, isPending, run } = createIssuerStep(query.get("error_description") ?? undefined);

    // sign in with a code, asking for the second factor where the person has one
    const complete = (otp: string) =>
        run(async () => {
            const body = { email: email(), otp };
            const signedIn = SignedIn.parse(
                await callIssuer("/sign-in/email-otp", environment, body),
            );
            if (signedIn.twoFactorRedirect === true) {
                setStep("second-factor");
            } else {
                environment.navigation.assign(next);
            }
        });

    // complete the code a mailed link carries, else offer the issuer's providers
    const [providers, setProviders] = createSignal<readonly ProviderId[]>([]);
    onSettled(() => {
        const linked = query.get("code");
        if (linked !== null) {
            setStep("code");
            complete(linked);
        } else {
            run(async () => {
                const listed = await callIssuer(SIGN_IN_PROVIDERS_PATH, environment);
                setProviders(SignInProviders.parse(listed).providers);
            });
        }
    });

    return (
        <CardPage title={locale.render(t`Sign in to Destack`)} error={error()}>
            <Switch>
                <Match when={step() === "email"}>
                    <For each={providers()}>
                        {(provider) => (
                            <SignInProvider
                                provider={provider}
                                isPending={isPending()}
                                onSelect={(chosen) =>
                                    run(async () => {
                                        // send the browser to the provider, returning to the continuation
                                        const body = {
                                            provider: chosen,
                                            callbackURL: next,
                                            errorCallbackURL: UNIVERSE_PAGES.signIn,
                                        };
                                        const { url } = Redirection.parse(
                                            await callIssuer("/sign-in/social", environment, body),
                                        );
                                        environment.navigation.assign(url);
                                    })
                                }
                            />
                        )}
                    </For>
                    <SignInEmail
                        isPending={isPending()}
                        onSubmit={(address) =>
                            run(async () => {
                                // mail a code, then ask for it
                                await callIssuer("/email-otp/send-verification-otp", environment, {
                                    email: address,
                                    type: "sign-in",
                                });
                                setEmail(address);
                                setStep("code");
                            })
                        }
                    />
                    <p>
                        {locale.render(t`New to Destack?`)}{" "}
                        <a href={`/sign-up?${query}`}>{locale.render(t`Create your Destack`)}</a>
                    </p>
                </Match>
                <Match when={step() === "code"}>
                    <SignInCode
                        email={email()}
                        isPending={isPending()}
                        onComplete={complete}
                        onResend={() =>
                            run(async () => {
                                // mail a new code
                                await callIssuer("/email-otp/send-verification-otp", environment, {
                                    email: email(),
                                    type: "sign-in",
                                });
                            })
                        }
                    />
                </Match>
                <Match when={step() === "second-factor"}>
                    <SignInSecondFactor
                        isPending={isPending()}
                        onSubmit={(code, kind) =>
                            run(async () => {
                                // answer the challenge, then continue
                                const path =
                                    kind === "authenticator"
                                        ? "/two-factor/verify-totp"
                                        : "/two-factor/verify-backup-code";
                                await callIssuer(path, environment, { code });
                                environment.navigation.assign(next);
                            })
                        }
                    />
                </Match>
            </Switch>
        </CardPage>
    );
}
