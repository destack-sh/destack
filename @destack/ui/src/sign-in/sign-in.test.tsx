import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import { flush } from "@destack/view";
import { SignInCode, SignInEmail, SignInProvider, SignInSecondFactor } from "./index.ts";

/** Type text into an input as a person or an autofill would, then let the view react. */
function enter(input: HTMLInputElement | null, text: string): void {
    if (input === null) {
        throw new TypeError("the input is missing");
    }
    input.value = text;
    if (input.type !== "email") {
        input.setSelectionRange(text.length, text.length);
    }
    input.dispatchEvent(new Event("input", { bubbles: true }));
    flush();
}

test("sign in by a provider, an email and its mailed code", () => {
    // render every first step, recording what each asks for
    const asked: string[] = [];
    const { container } = render(() => (
        <div>
            <SignInProvider
                provider="github"
                onSelect={(provider) => asked.push(`provider ${provider}`)}
            />
            <SignInEmail onSubmit={(email) => asked.push(`email ${email}`)} />
            <SignInCode
                email="ada@example.com"
                onComplete={(code) => asked.push(`code ${code}`)}
                onResend={() => asked.push("resend")}
            />
        </div>
    ));

    // choose GitHub, send a code to an email, enter the code, and ask for another
    const [provider, , resend] = [...container.querySelectorAll("button")];
    provider?.click();
    enter(container.querySelector("input[type=email]"), "  ada@example.com ");
    container
        .querySelector("form")
        ?.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
    enter(container.querySelector("input[autocomplete=one-time-code]"), "123456");
    resend?.click();

    // read the provider's and the code's labels, and what each step asked
    expect({
        provider: provider?.textContent,
        code: container.querySelectorAll("label")[1]?.textContent,
        asked,
    }).toEqual({
        provider: "Continue with \u2068GitHub\u2069",
        code: "Code sent to \u2068ada@example.com\u2069",
        asked: ["provider github", "email ada@example.com", "code 123456", "resend"],
    });
});

test("answer the second step with an authenticator's code, or switch to a recovery code", () => {
    // render the second step, recording each code and its kind
    const checked: string[] = [];
    const { container } = render(() => (
        <SignInSecondFactor onSubmit={(code, kind) => checked.push(`${kind} ${code}`)} />
    ));

    // enter an authenticator's code, then switch and submit a recovery code
    enter(container.querySelector("input[autocomplete=one-time-code]"), "654321");
    const toggle = [...container.querySelectorAll("button")].at(-1);
    const before = toggle?.textContent;
    toggle?.click();
    flush();
    enter(container.querySelector("input[name=code]"), " ABCD-EFGH ");
    container
        .querySelector("form")
        ?.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));

    expect({
        before,
        after: [...container.querySelectorAll("button")].at(-1)?.textContent,
        checked,
    }).toEqual({
        before: "Use a recovery code",
        after: "Use your authenticator app",
        checked: ["authenticator 654321", "recovery ABCD-EFGH"],
    });
});
