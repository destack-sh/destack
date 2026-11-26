import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { createSignal, Show } from "@destack/view";
import { Field, FieldLabel } from "../field/index.ts";
import { OtpField, OtpFieldGroup, OtpFieldSeparator, OtpFieldSlot } from "./otp-field.tsx";

/** The six-digit code a sign-in sends, confirmed once all digits are in. */
export const otpFieldSignInCode = defineExample({
    of: OtpField,
    name: "signInCode",
    description: t`The six-digit code a sign-in sends, confirmed once all digits are in.`,
    render: () => {
        const [code, setCode] = createSignal<string>();

        return (
            <Field>
                <FieldLabel>Sign-in code</FieldLabel>
                <OtpField maxLength={6} onComplete={setCode}>
                    <OtpFieldGroup>
                        <OtpFieldSlot index={0} />
                        <OtpFieldSlot index={1} />
                        <OtpFieldSlot index={2} />
                    </OtpFieldGroup>
                    <OtpFieldSeparator />
                    <OtpFieldGroup>
                        <OtpFieldSlot index={3} />
                        <OtpFieldSlot index={4} />
                        <OtpFieldSlot index={5} />
                    </OtpFieldGroup>
                </OtpField>
                <Show when={code()}>{(entered) => <p role="status">Checking {entered()}</p>}</Show>
            </Field>
        );
    },
});

/** The six-digit sign-in code marked invalid. */
export const otpFieldSignInCodeInvalid = defineExample({
    of: OtpField,
    name: "signInCodeInvalid",
    description: t`The six-digit sign-in code marked invalid.`,
    properties: {
        "aria-invalid": "true",
    },
    render: (properties) => (
        <Field>
            <FieldLabel>Sign-in code</FieldLabel>
            <OtpField {...properties} maxLength={6}>
                <OtpFieldGroup>
                    <OtpFieldSlot index={0} />
                    <OtpFieldSlot index={1} />
                    <OtpFieldSlot index={2} />
                </OtpFieldGroup>
                <OtpFieldSeparator />
                <OtpFieldGroup>
                    <OtpFieldSlot index={3} />
                    <OtpFieldSlot index={4} />
                    <OtpFieldSlot index={5} />
                </OtpFieldGroup>
            </OtpField>
        </Field>
    ),
});

/** The six-digit sign-in code unavailable. */
export const otpFieldSignInCodeDisabled = defineExample({
    of: OtpField,
    name: "signInCodeDisabled",
    description: t`The six-digit sign-in code unavailable.`,
    properties: {
        disabled: true,
    },
    render: (properties) => (
        <Field>
            <FieldLabel>Sign-in code</FieldLabel>
            <OtpField {...properties} maxLength={6}>
                <OtpFieldGroup>
                    <OtpFieldSlot index={0} />
                    <OtpFieldSlot index={1} />
                    <OtpFieldSlot index={2} />
                </OtpFieldGroup>
                <OtpFieldSeparator />
                <OtpFieldGroup>
                    <OtpFieldSlot index={3} />
                    <OtpFieldSlot index={4} />
                    <OtpFieldSlot index={5} />
                </OtpFieldGroup>
            </OtpField>
        </Field>
    ),
});
