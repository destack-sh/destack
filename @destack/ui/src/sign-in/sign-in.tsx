import { Icon } from "@destack/icon";
import githubLogo from "@destack/icon/phosphor/github-logo";
import googleLogo from "@destack/icon/phosphor/google-logo";
import windowsLogo from "@destack/icon/phosphor/windows-logo";
import { argument, t } from "@destack/locale";
import { createSignal, type JSX, Show, useLocale } from "@destack/view";
import { Button } from "../button/index.ts";
import { Field, FieldLabel } from "../field/index.ts";
import { Input } from "../input/index.ts";
import { InputOTP, InputOTPGroup, InputOTPSeparator, InputOTPSlot } from "../input-otp/index.ts";
import { Stack } from "../stack/index.ts";

/** The digits of a mailed sign-in code and of an authenticator's code (RFC 6238). */
const CODE_LENGTH = 6;

/** The providers a person signs in with, by their identifier: the name they read and their mark. */
export const SIGN_IN_PROVIDERS = {
    github: { name: "GitHub", icon: githubLogo },
    google: { name: "Google", icon: googleLogo },
    microsoft: { name: "Microsoft", icon: windowsLogo },
} as const;

/** A provider a person signs in with. */
export type SignInProviderId = keyof typeof SIGN_IN_PROVIDERS;

/** The properties of a provider's button. */
export interface SignInProviderProperties {
    /** The provider. */
    readonly provider: SignInProviderId;
    /** Whether a sign-in is on its way, which disables the button. */
    readonly isPending?: boolean | undefined;
    /** Start signing in with the provider. */
    readonly onSelect: (provider: SignInProviderId) => void;
}

/** Render the button signing in with a provider, by its mark and name. */
export function SignInProvider(properties: SignInProviderProperties): JSX.Element {
    const locale = useLocale();
    const provider = () => SIGN_IN_PROVIDERS[properties.provider];

    return (
        <Button
            variant="outline"
            disabled={properties.isPending === true}
            onClick={() => properties.onSelect(properties.provider)}
        >
            <Icon icon={provider().icon} />
            {locale.render(t`Continue with ${argument("provider")}`, { provider: provider().name })}
        </Button>
    );
}

/** The properties of the email step. */
export interface SignInEmailProperties {
    /** Whether the code is on its way, which disables the form. */
    readonly isPending?: boolean | undefined;
    /** Send the code to an email. */
    readonly onSubmit: (email: string) => void;
}

/** Render the step asking for the email a code goes to. */
export function SignInEmail(properties: SignInEmailProperties): JSX.Element {
    const locale = useLocale();
    const [email, setEmail] = createSignal("");

    return (
        <form
            onSubmit={(event) => {
                // send the code instead of posting the form
                event.preventDefault();
                properties.onSubmit(email().trim());
            }}
        >
            <Stack space="3">
                <Field>
                    <FieldLabel>{locale.render(t`Email`)}</FieldLabel>
                    <Input
                        type="email"
                        name="email"
                        autocomplete="email"
                        required
                        value={email()}
                        onInput={(event) => setEmail(event.currentTarget.value)}
                    />
                </Field>
                <Button type="submit" disabled={properties.isPending === true}>
                    {locale.render(t`Continue with email`)}
                </Button>
            </Stack>
        </form>
    );
}

/** The properties of the code step. */
export interface SignInCodeProperties {
    /** The email the code went to. */
    readonly email: string;
    /** Whether the code is being checked, which disables the input. */
    readonly isPending?: boolean | undefined;
    /** Check the code once all its digits are in. */
    readonly onComplete: (code: string) => void;
    /** Send a new code. */
    readonly onResend: () => void;
}

/** Render the step asking for the mailed code, checked once all its digits are in. */
export function SignInCode(properties: SignInCodeProperties): JSX.Element {
    const locale = useLocale();

    return (
        <Stack space="3">
            <Field>
                <FieldLabel>
                    {locale.render(t`Code sent to ${argument("email")}`, {
                        email: properties.email,
                    })}
                </FieldLabel>
                <CodeInput
                    isPending={properties.isPending === true}
                    onComplete={properties.onComplete}
                />
            </Field>
            <Button variant="link" onClick={() => properties.onResend()}>
                {locale.render(t`Send a new code`)}
            </Button>
        </Stack>
    );
}

/** The properties of the second step. */
export interface SignInSecondFactorProperties {
    /** Whether the code is being checked, which disables the input. */
    readonly isPending?: boolean | undefined;
    /** Check an authenticator's code or a recovery code. */
    readonly onSubmit: (code: string, kind: "authenticator" | "recovery") => void;
}

/** Render the second step: the authenticator's code, or a recovery code instead. */
export function SignInSecondFactor(properties: SignInSecondFactorProperties): JSX.Element {
    // ask for the authenticator's code until the person switches to a recovery code
    const locale = useLocale();
    const [isRecovery, setRecovery] = createSignal(false);
    const [recovery, setRecoveryCode] = createSignal("");

    return (
        <Stack space="3">
            <Show
                when={isRecovery()}
                fallback={
                    <Field>
                        <FieldLabel>
                            {locale.render(t`Code from your authenticator app`)}
                        </FieldLabel>
                        <CodeInput
                            isPending={properties.isPending === true}
                            onComplete={(code) => properties.onSubmit(code, "authenticator")}
                        />
                    </Field>
                }
            >
                <form
                    onSubmit={(event) => {
                        // check the recovery code instead of posting the form
                        event.preventDefault();
                        properties.onSubmit(recovery().trim(), "recovery");
                    }}
                >
                    <Stack space="3">
                        <Field>
                            <FieldLabel>{locale.render(t`Recovery code`)}</FieldLabel>
                            <Input
                                name="code"
                                autocomplete="one-time-code"
                                required
                                value={recovery()}
                                onInput={(event) => setRecoveryCode(event.currentTarget.value)}
                            />
                        </Field>
                        <Button type="submit" disabled={properties.isPending === true}>
                            {locale.render(t`Verify`)}
                        </Button>
                    </Stack>
                </form>
            </Show>
            <Button variant="link" onClick={() => setRecovery(!isRecovery())}>
                {isRecovery()
                    ? locale.render(t`Use your authenticator app`)
                    : locale.render(t`Use a recovery code`)}
            </Button>
        </Stack>
    );
}

/** Render six digits in two groups of three, as mailed and authenticator codes are read. */
function CodeInput(properties: {
    readonly isPending: boolean;
    readonly onComplete: (code: string) => void;
}): JSX.Element {
    return (
        <InputOTP
            maxLength={CODE_LENGTH}
            autocomplete="one-time-code"
            autofocus
            disabled={properties.isPending}
            onComplete={properties.onComplete}
        >
            <InputOTPGroup>
                <InputOTPSlot index={0} />
                <InputOTPSlot index={1} />
                <InputOTPSlot index={2} />
            </InputOTPGroup>
            <InputOTPSeparator />
            <InputOTPGroup>
                <InputOTPSlot index={3} />
                <InputOTPSlot index={4} />
                <InputOTPSlot index={5} />
            </InputOTPGroup>
        </InputOTP>
    );
}
