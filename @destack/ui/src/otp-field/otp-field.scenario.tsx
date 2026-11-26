import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { otpFieldSignInCode } from "./otp-field.example.tsx";
import { OtpField } from "./otp-field.tsx";

/** Fill a code's slots as the person types and confirm it once complete. */
export const otpFieldFillSlots = defineScenario({
    of: OtpField,
    interaction: viewInteraction,
    name: "fillSlots",
    description: t`Show each typed digit in its slot and report the code once every slot has one.`,
    given: { examples: [otpFieldSignInCode] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { label: "Sign-in code" } },
            then: [
                {
                    observe: { kind: "texts", target: { css: "[data-slot=otp-field-slot]" } },
                    is: ["", "", "", "", "", ""],
                },
                { observe: { kind: "texts", target: { role: "status" } }, is: [] },
            ],
        },
        {
            step: { action: "fill", target: { label: "Sign-in code" }, value: "12" },
            then: [
                {
                    observe: { kind: "texts", target: { css: "[data-slot=otp-field-slot]" } },
                    is: ["1", "2", "", "", "", ""],
                },
                { observe: { kind: "texts", target: { role: "status" } }, is: [] },
            ],
        },
        {
            step: { action: "fill", target: { label: "Sign-in code" }, value: "123456" },
            then: [
                {
                    observe: { kind: "texts", target: { css: "[data-slot=otp-field-slot]" } },
                    is: ["1", "2", "3", "4", "5", "6"],
                },
                { observe: { kind: "texts", target: { role: "status" } }, is: ["Checking 123456"] },
            ],
        },
    ],
});
