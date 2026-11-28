import { Failure } from "@destack/error";
import { t } from "@destack/locale";
import { expect, test } from "@destack/test";
import { defineView } from "../declare/view.ts";
import type { JSX } from "../solid/component.ts";
import { createMemo } from "../solid/reactive.ts";
import { changed, render } from "../test/dom.ts";
import type { ErrorFallbackProperties } from "../declare/view.ts";
import { FALLBACKS } from "./fallback.tsx";

/** The longest a test waits for a render, far above the few microtasks a local resource takes. */
const RENDER_MILLISECONDS = 2000;

/** Fail as a missing page does. */
function Missing(): JSX.Element {
    throw Failure.error(Failure.http("NOT_FOUND", "page 'x' not found"));
}

/** Fail as a page whose session ended does. */
function SignedOut(): JSX.Element {
    throw Failure.error(Failure.http("UNAUTHORIZED", "authentication required: the session ended"));
}

/** Show the failure's code, as a view's own error part may. */
function Code(properties: ErrorFallbackProperties): JSX.Element {
    return <p>{properties.failure.code}</p>;
}

test("draw a failed view with its own error part, else with the runtime's and its way out", async () => {
    // declare a missing page with an error part of its own, and one without
    const own = defineView({
        name: "own",
        title: t`Own`,
        component: async () => ({ default: Missing }),
        error: async () => ({ default: Code }),
    });
    const plain = defineView({
        name: "plain",
        title: t`Plain`,
        component: async () => ({ default: Missing }),
    });
    const ended = defineView({
        name: "ended",
        title: t`Ended`,
        component: async () => ({ default: SignedOut }),
    });

    // draw all three
    const Own = await own.load(FALLBACKS);
    const Plain = await plain.load(FALLBACKS);
    const Ended = await ended.load(FALLBACKS);
    const drawn = [render(() => <Own />), render(() => <Plain />), render(() => <Ended />)];

    // show the own part, the default without a way out, and the default offering to sign in
    expect(drawn.map(({ container }) => container.textContent)).toEqual([
        "NOT_FOUND",
        "Nothing hereThere is no page at this address, or it moved.",
        "Sign in to continueYour session ended, so sign in again.Sign in",
    ]);
});

test("draw a view again on retry after a failure that may pass", async () => {
    // fail the first draw as an unavailable service does, then draw
    let draws = 0;
    const Flaky = (): JSX.Element => {
        draws += 1;
        if (draws === 1) {
            throw Failure.error(Failure.http("SERVICE_UNAVAILABLE", "app 'tasks' is starting"));
        }

        return <p>Ready</p>;
    };
    const flaky = defineView({
        name: "flaky",
        title: t`Flaky`,
        component: async () => ({ default: Flaky }),
    });

    // try again from the runtime's error part
    const Root = await flaky.load(FALLBACKS);
    const { container } = render(() => <Root />);
    const failed = container.textContent;
    container.querySelector("button")?.click();
    await changed(container, () => container.textContent === "Ready", RENDER_MILLISECONDS);

    expect([failed, container.textContent]).toEqual([
        "Not available right nowIt should be back in a moment.Try again",
        "Ready",
    ]);
});

test("hold a view's place while its data is pending, then draw it", async () => {
    // read a value that arrives once released
    const { promise: arriving, resolve: release } = Promise.withResolvers<string>();
    const Pending = (): JSX.Element => {
        const value = createMemo(async () => await arriving);

        return <p>{value()}</p>;
    };
    const pending = defineView({
        name: "pending",
        title: t`Pending`,
        component: async () => ({ default: Pending }),
    });

    // draw it before and after the value arrives
    const Root = await pending.load(FALLBACKS);
    const { container } = render(() => <Root />);
    const busy = container.querySelector("main")?.getAttribute("aria-busy");
    release("Ready");
    await changed(container, () => container.textContent === "Ready", RENDER_MILLISECONDS);

    // hold the place with the runtime's empty, busy part
    expect([busy, container.textContent]).toEqual(["true", "Ready"]);
});
