import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import {
    ConsentPrompt,
    Grant,
    GrantList,
    Permission,
    PermissionGroup,
    PermissionList,
} from "./index.ts";

test("list what an application asks for by place, and allow it, allow it once, or refuse it", () => {
    // render a request in two places, one permission allowed before, recording each answer
    const answers: string[] = [];
    const { container } = render(() => (
        <ConsentPrompt
            onAllow={() => answers.push("allow")}
            onAllowOnce={() => answers.push("once")}
            onDeny={() => answers.push("deny")}
        >
            <PermissionList>
                <PermissionGroup title="In the space">
                    <Permission description="See and change notes" />
                </PermissionGroup>
                <PermissionGroup title="In your home">
                    <Permission description="See settings" isGranted />
                </PermissionGroup>
            </PermissionList>
        </ConsentPrompt>
    ));

    // answer with each button in turn
    const buttons = [...container.querySelectorAll("button")];
    buttons.forEach((button) => button.click());

    expect({
        groups: [...container.querySelectorAll('[role="group"]')].map((group) => [
            group.getAttribute("aria-label"),
            [...group.querySelectorAll("li")].map((permission) => permission.textContent),
        ]),
        buttons: buttons.map((button) => button.textContent),
        answers,
    }).toEqual({
        groups: [
            ["In the space", ["See and change notes"]],
            ["In your home", ["See settingsAlready allowed"]],
        ],
        buttons: ["Allow", "Only this time", "Don't allow"],
        answers: ["allow", "once", "deny"],
    });
});

test("offer only a lasting answer when no answer for this time fits", () => {
    const { container } = render(() => (
        <ConsentPrompt onAllow={() => undefined} onDeny={() => undefined}>
            <PermissionList>
                <PermissionGroup title="Your account">
                    <Permission description="Know who you are" />
                </PermissionGroup>
            </PermissionList>
        </ConsentPrompt>
    ));

    expect([...container.querySelectorAll("button")].map((button) => button.textContent)).toEqual([
        "Allow",
        "Don't allow",
    ]);
});

test("list the applications a person lets act for them with what each may do, and remove one", () => {
    // render two grants, recording each removal
    const removed: string[] = [];
    const { container } = render(() => (
        <GrantList>
            <Grant name="Notes" description="In work--ada" onRemove={() => removed.push("notes")}>
                <PermissionList>
                    <PermissionGroup title="In the space">
                        <Permission description="See and change notes" />
                    </PermissionGroup>
                </PermissionList>
            </Grant>
            <Grant
                name="Tasks"
                description="In team--acme"
                onRemove={() => removed.push("tasks")}
            />
        </GrantList>
    ));

    // remove the second
    const items = [...container.querySelectorAll('[role="listitem"]')];
    items[1]?.querySelector("button")?.click();

    expect({
        items: items.map((item) => [
            item.querySelector('[data-slot="item-title"]')?.textContent,
            item.querySelector('[data-slot="item-description"]')?.textContent,
            [...item.querySelectorAll("li")].map((permission) => permission.textContent),
        ]),
        removed,
    }).toEqual({
        items: [
            ["Notes", "In work--ada", ["See and change notes"]],
            ["Tasks", "In team--acme", []],
        ],
        removed: ["tasks"],
    });
});
