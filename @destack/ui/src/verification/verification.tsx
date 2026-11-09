import { argument, t } from "@destack/locale";
import { createSignal, type JSX, Match, Switch, useLocale } from "@destack/view";
import { Button } from "../button/index.ts";
import { Field, FieldLabel } from "../field/index.ts";
import { Input } from "../input/index.ts";
import { Stack } from "../stack/index.ts";

/** A device's sign-in its code found: the code, the application asking and where it stands. */
export interface VerificationRequest {
    /** The code the device shows. */
    readonly userCode: string;
    /** The name of the application on the device. */
    readonly client: string;
    /** Whether the person decided it yet, and how. */
    readonly status: "pending" | "approved" | "denied";
}

/** The properties of a device's verification: the sign-in its code found and the answer. */
export interface VerificationProperties {
    /** The sign-in the code found, absent while the code is being entered. */
    readonly request?: VerificationRequest | undefined;
    /** Whether a step is on its way, which disables the buttons. */
    readonly isPending?: boolean | undefined;
    /** Look a code up. */
    readonly onSubmit: (userCode: string) => void;
    /** Sign the device in. */
    readonly onApprove: (request: VerificationRequest) => void;
    /** Refuse it. */
    readonly onDeny: (request: VerificationRequest) => void;
}

/** Render a device's sign-in by the code it shows: enter it, then approve or deny the device. */
export function Verification(properties: VerificationProperties): JSX.Element {
    // keep the typed code
    const locale = useLocale();
    const [code, setCode] = createSignal("");
    const isPending = () => properties.isPending === true;

    return (
        <Stack space="4">
            <Switch>
                <Match when={properties.request === undefined}>
                    <form
                        onSubmit={(event) => {
                            // look the code up instead of posting the form
                            event.preventDefault();
                            properties.onSubmit(code().trim().toUpperCase());
                        }}
                    >
                        <Stack space="3">
                            <Field>
                                <FieldLabel>
                                    {locale.render(t`Code shown on your device`)}
                                </FieldLabel>
                                <Input
                                    name="user_code"
                                    autocomplete="off"
                                    required
                                    value={code()}
                                    onInput={(event) => setCode(event.currentTarget.value)}
                                />
                            </Field>
                            <Button type="submit" disabled={isPending()}>
                                {locale.render(t`Continue`)}
                            </Button>
                        </Stack>
                    </form>
                </Match>
                <Match
                    when={properties.request?.status === "pending" ? properties.request : undefined}
                >
                    {(request) => (
                        <>
                            <p>
                                {locale.render(
                                    t`Sign in to ${argument("client")} with the code ${argument("code")}?`,
                                    {
                                        client: request().client,
                                        code: request().userCode,
                                    },
                                )}
                            </p>
                            <Stack space="2">
                                <Button
                                    disabled={isPending()}
                                    onClick={() => properties.onApprove(request())}
                                >
                                    {locale.render(t`Approve`)}
                                </Button>
                                <Button
                                    variant="outline"
                                    disabled={isPending()}
                                    onClick={() => properties.onDeny(request())}
                                >
                                    {locale.render(t`Deny`)}
                                </Button>
                            </Stack>
                        </>
                    )}
                </Match>
                <Match when={properties.request?.status === "approved"}>
                    <p>{locale.render(t`Device signed in. Return to it to continue.`)}</p>
                </Match>
                <Match when={properties.request?.status === "denied"}>
                    <p>{locale.render(t`Device not signed in.`)}</p>
                </Match>
            </Switch>
        </Stack>
    );
}
