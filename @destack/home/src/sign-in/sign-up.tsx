import { TimeZone } from "@destack/schema";
import {
    HANDLE_CHECK_PATH,
    HandleCheck,
    RESIDENCY_CHOICE_PATH,
    ResidencyChoice,
} from "@destack/account/authentication";
import { Field, FieldGroup, FieldLabel } from "@destack/ui/field";
import { Input } from "@destack/ui/input";
import { Button } from "@destack/ui/button";
import { SignInCode } from "@destack/ui/sign-in";
import {
    type HandleAvailability,
    SignUpHandle,
    SignUpResidency,
    type SignUpResidencyOption,
} from "@destack/ui/sign-up";
import { t } from "@destack/locale";
import { createSignal, onSettled, Show, useEnvironment, useLocale } from "@destack/view";
import { CardPage } from "./page.tsx";
import { callIssuer, continuation, createIssuerStep } from "./issuer.ts";

/** Sign a person up: their name, handle and residency, then the mailed code that creates them. */
export default function SignUpPage() {
    // read the page's query and the browser's time zone, which the person never types
    const environment = useEnvironment();
    const locale = useLocale();
    const query = new URL(environment.navigation.currentEntry.url).searchParams;
    const timeZone = TimeZone.parse(Intl.DateTimeFormat().resolvedOptions().timeZone);
    const [email, setEmail] = createSignal(query.get("email") ?? "");
    const [name, setName] = createSignal("");
    const [handle, setHandle] = createSignal("");
    const [availability, setAvailability] = createSignal<HandleAvailability>();
    const [residencies, setResidencies] = createSignal<readonly SignUpResidencyOption[]>([]);
    const [residency, setResidency] = createSignal<string>();
    const [isSent, setSent] = createSignal(false);

    // run each step at the issuer, showing its refusal
    const { error, isPending, run } = createIssuerStep();

    // read the residencies with the one the visitor's country is offered
    onSettled(() =>
        run(async () => {
            const read = ResidencyChoice.parse(
                await callIssuer(RESIDENCY_CHOICE_PATH, environment),
            );
            setResidencies(read.residencies);
            setResidency(read.suggested ?? undefined);
        }),
    );

    // check each typed handle, keeping only the latest answer
    const check = (typed: string) => {
        setHandle(typed);
        setAvailability(undefined);
        run(async () => {
            const path = `${HANDLE_CHECK_PATH}?${new URLSearchParams({ handle: typed })}`;
            const checked = HandleCheck.parse(await callIssuer(path, environment));
            if (checked.handle === handle()) {
                setAvailability(checked.availability);
            }
        });
    };

    // create the person and their account with a code, then continue
    const complete = (otp: string) =>
        run(async () => {
            await callIssuer("/sign-in/email-otp", environment, {
                email: email(),
                otp,
                name: name(),
                handle: handle(),
                residencyId: residency(),
                timeZone,
                locale: locale.tag,
            });
            environment.navigation.assign(continuation(query));
        });

    // complete the code a mailed link carries, else mail one
    const send = () => {
        const linked = query.get("code");
        if (linked !== null) {
            complete(linked);
        } else {
            run(async () => {
                const body = { email: email(), type: "sign-in" };
                await callIssuer("/email-otp/send-verification-otp", environment, body);
                setSent(true);
            });
        }
    };

    return (
        <CardPage title={locale.render(t`Create your Destack`)} error={error()}>
            <Show
                when={isSent()}
                fallback={
                    <form
                        onSubmit={(event) => {
                            // mail the code instead of posting the form
                            event.preventDefault();
                            send();
                        }}
                    >
                        <FieldGroup>
                            <Field>
                                <FieldLabel>{locale.render(t`Email`)}</FieldLabel>
                                <Input
                                    type="email"
                                    autocomplete="email"
                                    required
                                    value={email()}
                                    onInput={(event) => setEmail(event.currentTarget.value.trim())}
                                />
                            </Field>
                            <Field>
                                <FieldLabel>{locale.render(t`Name`)}</FieldLabel>
                                <Input
                                    autocomplete="name"
                                    required
                                    value={name()}
                                    onInput={(event) => setName(event.currentTarget.value)}
                                />
                            </Field>
                            <SignUpHandle
                                value={handle()}
                                availability={availability()}
                                onInput={check}
                            />
                            <SignUpResidency
                                residencies={residencies()}
                                value={residency()}
                                onChange={setResidency}
                            />
                            <Button
                                type="submit"
                                disabled={
                                    isPending() ||
                                    availability() !== "available" ||
                                    residency() === undefined
                                }
                            >
                                {query.has("code")
                                    ? locale.render(t`Create your Destack`)
                                    : locale.render(t`Email me a code`)}
                            </Button>
                        </FieldGroup>
                    </form>
                }
            >
                <SignInCode
                    email={email()}
                    isPending={isPending()}
                    onResend={send}
                    onComplete={complete}
                />
            </Show>
        </CardPage>
    );
}
