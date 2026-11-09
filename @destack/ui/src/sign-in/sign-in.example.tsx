import { defineExample } from "@destack/package/declare";
import { createSignal, Match, Switch } from "@destack/view";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../card/index.ts";
import { Stack } from "../stack/index.ts";
import { SignInCode, SignInEmail, SignInProvider } from "./sign-in.tsx";

/** A sign-in page by a provider or a mailed code, the code step once an email was sent. */
export const signInByCode = defineExample({
    of: SignInEmail,
    name: "by-code",
    description:
        "a sign-in page by a provider or a mailed code, the code step once an email was sent",
    render: () => {
        const [email, setEmail] = createSignal<string>();

        return (
            <Card>
                <CardHeader>
                    <CardTitle render={(attributes) => <h1 {...attributes} />}>
                        Sign in to Destack
                    </CardTitle>
                    <CardDescription>Use your email or an account you have.</CardDescription>
                </CardHeader>
                <CardContent>
                    <Switch>
                        <Match when={email()}>
                            {(sent) => (
                                <SignInCode
                                    email={sent()}
                                    onComplete={() => undefined}
                                    onResend={() => undefined}
                                />
                            )}
                        </Match>
                        <Match when={email() === undefined}>
                            <Stack space="3">
                                <SignInProvider provider="github" onSelect={() => undefined} />
                                <SignInProvider provider="google" onSelect={() => undefined} />
                                <SignInProvider provider="microsoft" onSelect={() => undefined} />
                                <SignInEmail onSubmit={setEmail} />
                            </Stack>
                        </Match>
                    </Switch>
                </CardContent>
            </Card>
        );
    },
});
