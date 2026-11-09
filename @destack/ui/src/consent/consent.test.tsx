import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import { Consent, ConsentScope } from "./index.ts";

test("list the scopes an application asks for, and allow or deny them", () => {
    // render a request for two scopes, recording each answer
    const answers: string[] = [];
    const { container } = render(() => (
        <Consent onApprove={() => answers.push("approve")} onDeny={() => answers.push("deny")}>
            <ConsentScope description="Know who you are" />
            <ConsentScope description="Stay signed in" />
        </Consent>
    ));

    // allow, then deny
    const [allow, deny] = [...container.querySelectorAll("button")];
    allow?.click();
    deny?.click();

    expect({
        scopes: [...container.querySelectorAll("li")].map((scope) => scope.textContent),
        buttons: [allow?.textContent, deny?.textContent],
        answers,
    }).toEqual({
        scopes: ["Know who you are", "Stay signed in"],
        buttons: ["Allow", "Deny"],
        answers: ["approve", "deny"],
    });
});
