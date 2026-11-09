import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import { createSignal, flush } from "@destack/view";
import { Verification, type VerificationRequest } from "./index.ts";

test("look a typed device code up, then approve the device, and say it is signed in", () => {
    // render the verification while the code is being entered, recording each step
    const [request, setRequest] = createSignal<VerificationRequest>();
    const steps: string[] = [];
    const { container } = render(() => (
        <Verification
            request={request()}
            onSubmit={(code) => steps.push(`submit ${code}`)}
            onApprove={() => steps.push("approve")}
            onDeny={() => steps.push("deny")}
        />
    ));

    // type the code loosely and submit it
    const input = container.querySelector("input");
    if (input !== null) {
        input.value = " wttc-ppvx ";
        input.dispatchEvent(new Event("input", { bubbles: true }));
        flush();
    }
    container
        .querySelector("form")
        ?.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));

    // approve the pending sign-in the code found
    setRequest({ userCode: "WTTC-PPVX", client: "Destack CLI", status: "pending" });
    flush();
    const question = container.querySelector("p")?.textContent;
    container.querySelector("button")?.click();
    setRequest({ userCode: "WTTC-PPVX", client: "Destack CLI", status: "approved" });
    flush();

    expect({ steps, question, done: container.querySelector("p")?.textContent }).toEqual({
        steps: ["submit WTTC-PPVX", "approve"],
        question: "Sign in to ⁨Destack CLI⁩ with the code ⁨WTTC-PPVX⁩?",
        done: "Device signed in. Return to it to continue.",
    });
});
