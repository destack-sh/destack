import { expect, test } from "@destack/test";
import { MOTION_SCALE } from "@destack/theme";
import { startViewTransition } from "./index.ts";

test("morph the given elements in the scope's own transition, naming them for it alone", async () => {
    // give the scope a transition of its own, recording each step
    const scope = document.createElement("div");
    const items = [document.createElement("span"), document.createElement("span")];
    scope.append(...items);
    document.body.append(scope);
    const steps: string[] = [];
    const names = (): string => items.map((item) => item.style.viewTransitionName).join(",");
    Object.assign(scope, {
        startViewTransition: (update: () => void) => {
            steps.push(`start ${names()}`);
            update();

            return { ready: Promise.resolve(), finished: Promise.resolve() };
        },
    });
    Object.assign(document, { startViewTransition: () => steps.push("page") });

    // run a change, then read the names once it finishes
    await startViewTransition(scope, () => steps.push(`update ${names()}`), items);
    steps.push(`finished ${names()}`);

    // the scope ran it, the elements named as themselves until it finished
    expect(steps).toEqual([
        "start match-element,match-element",
        "update match-element,match-element",
        "finished ,",
    ]);
    Reflect.deleteProperty(document, "startViewTransition");
    scope.remove();
});

test("change at once where the person wants less motion", async () => {
    const scope = document.createElement("div");
    scope.style.setProperty(MOTION_SCALE, "0");
    document.body.append(scope);
    const steps: string[] = [];
    Object.assign(scope, { startViewTransition: () => steps.push("transition") });
    Object.assign(document, { startViewTransition: () => steps.push("page") });

    await startViewTransition(scope, () => steps.push("update"), []);

    expect(steps).toEqual(["update"]);
    Reflect.deleteProperty(document, "startViewTransition");
    scope.remove();
});
