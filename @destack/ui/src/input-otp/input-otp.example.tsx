import { defineExample } from "@destack/package/declare";
import { createSignal, Show } from "@destack/view";
import { Field, FieldLabel } from "../field/index.ts";
import { InputOTP, InputOTPGroup, InputOTPSeparator, InputOTPSlot } from "./input-otp.tsx";

/** The six-digit code a sign-in sends, confirmed once all digits are in. */
export const inputOTPSignInCode = defineExample({
    of: InputOTP,
    name: "sign-in-code",
    description: "the six-digit code a sign-in sends, confirmed once all digits are in",
    render: () => {
        const [code, setCode] = createSignal<string>();

        return (
            <Field>
                <FieldLabel>Sign-in code</FieldLabel>
                <InputOTP maxLength={6} onComplete={setCode}>
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
                <Show when={code()}>{(entered) => <p role="status">Checking {entered()}</p>}</Show>
            </Field>
        );
    },
});

/** The six-digit sign-in code marked invalid. */
export const inputOTPSignInCodeInvalid = defineExample({
    of: InputOTP,
    name: "sign-in-code-invalid",
    description: "the six-digit sign-in code marked invalid",
    properties: {
        "aria-invalid": "true",
    },
    render: (properties) => (
        <Field>
            <FieldLabel>Sign-in code</FieldLabel>
            <InputOTP {...properties} maxLength={6}>
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
        </Field>
    ),
});

/** The six-digit sign-in code unavailable. */
export const inputOTPSignInCodeDisabled = defineExample({
    of: InputOTP,
    name: "sign-in-code-disabled",
    description: "the six-digit sign-in code unavailable",
    properties: {
        disabled: true,
    },
    render: (properties) => (
        <Field>
            <FieldLabel>Sign-in code</FieldLabel>
            <InputOTP {...properties} maxLength={6}>
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
        </Field>
    ),
});
