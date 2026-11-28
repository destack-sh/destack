import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import { Refusal, type RefusalProperties } from "./index.ts";

/** The app the refusals are of. */
const TASKS = { name: "tasks", title: "Tasks" };

/** Where a person goes home to. */
const HOME = "https://home.example.test/";

/** Each refusal, with the gateway's next step where it gives one. */
const REFUSALS: readonly RefusalProperties[] = [
    { kind: "starting", package: TASKS, action: { kind: "retry", after: 60_000 } },
    {
        kind: "failed",
        package: TASKS,
        error: { reason: "INSTANCE_FAILED", message: "the module threw" },
        action: { kind: "home", url: HOME },
    },
    { kind: "not-found", action: { kind: "home", url: HOME } },
    { kind: "forbidden", package: TASKS, action: { kind: "home", url: HOME } },
    {
        kind: "signed-out",
        package: TASKS,
        action: { kind: "sign-in", url: "https://example.test/sign-in" },
    },
    { kind: "offline", action: { kind: "retry", after: 0 } },
];

test("say why each refusal happened and offer its next steps, the error under its details", () => {
    // a starting app waits on its own, a failed one offers a retry beside the way home
    expect(REFUSALS.map(pageOf)).toEqual([
        { heading: "Starting Tasks", steps: [] },
        { heading: "Tasks couldn't start", steps: ["Try again", `Go home → ${HOME}`, "Details"] },
        { heading: "Nothing here", steps: [`Go home → ${HOME}`] },
        { heading: "You don't have access to Tasks", steps: [`Go home → ${HOME}`] },
        { heading: "Sign in to continue", steps: ["Sign in → https://example.test/sign-in"] },
        { heading: "You're offline", steps: ["Try again"] },
    ]);
});

/** Read a text without the bidi isolates the locale wraps around interpolations. */
function textOf(text: string | undefined): string | undefined {
    return text?.replaceAll(/[\u2068\u2069]/gu, "");
}

/** Render a refusal, and read its heading, then each step's name and where a link leads. */
function pageOf(refusal: RefusalProperties) {
    const { container, unmount } = render(() => <Refusal {...refusal} />);
    const page = {
        heading: textOf(container.querySelector("h1")?.textContent),
        steps: [...container.querySelectorAll("button, a")].map((step) =>
            step instanceof HTMLAnchorElement
                ? `${step.textContent.trim()} → ${step.href}`
                : step.textContent.trim(),
        ),
    };
    unmount();

    return page;
}
