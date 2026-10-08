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
    when: [
        { action: "focus", target: { label: "Sign-in code" } },
        { action: "fill", target: { label: "Sign-in code" }, value: "12" },
        { action: "fill", target: { label: "Sign-in code" }, value: "123456" },
    ],
    then: {
        observe: {
            slots: { kind: "texts", target: { css: "[data-slot=input-otp-slot]" } },
            status: { kind: "texts", target: { role: "status" } },
        },
        each: [
            { slots: ["", "", "", "", "", ""], status: [] },
            { slots: ["1", "2", "", "", "", ""], status: [] },
            { slots: ["1", "2", "3", "4", "5", "6"], status: ["Checking 123456"] },
        ],
    },
});
