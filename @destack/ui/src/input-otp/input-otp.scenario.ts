import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { inputOTPSignInCode } from "./input-otp.example.tsx";
import { InputOTP } from "./input-otp.tsx";

/** Fill a code's slots as the person types and confirm it once complete. */
export const inputOTPFillSlots = defineScenario({
    of: InputOTP,
    interaction: viewInteraction,
    name: "fill-slots",
    description: "show each typed digit in its slot and report the code once every slot holds one",
    given: { examples: [inputOTPSignInCode] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { label: "Sign-in code" } },
            then: [
                {
                    observe: { kind: "texts", target: { css: "[data-slot=input-otp-slot]" } },
                    is: ["", "", "", "", "", ""],
                },
                { observe: { kind: "texts", target: { role: "status" } }, is: [] },
            ],
        },
        {
            step: { action: "fill", target: { label: "Sign-in code" }, value: "12" },
            then: [
                {
                    observe: { kind: "texts", target: { css: "[data-slot=input-otp-slot]" } },
                    is: ["1", "2", "", "", "", ""],
                },
                { observe: { kind: "texts", target: { role: "status" } }, is: [] },
            ],
        },
        {
            step: { action: "fill", target: { label: "Sign-in code" }, value: "123456" },
            then: [
                {
                    observe: { kind: "texts", target: { css: "[data-slot=input-otp-slot]" } },
                    is: ["1", "2", "3", "4", "5", "6"],
                },
                { observe: { kind: "texts", target: { role: "status" } }, is: ["Checking 123456"] },
            ],
        },
    ],
});
