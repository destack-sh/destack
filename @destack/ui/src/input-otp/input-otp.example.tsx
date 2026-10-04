import type { JSX } from "@solidjs/web";
import { createSignal, Show } from "solid-js";
import { Field, FieldLabel } from "../field/index.ts";
import { InputOTP, InputOTPGroup, InputOTPSeparator, InputOTPSlot } from "./input-otp.tsx";

/** Show the six-digit code a sign-in sends, confirmed once all digits are in. */
export function InputOTPExample(): JSX.Element {
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
}
