import { expect, test } from "@destack/test";
import { untouched } from "../test/server.ts";
import {
    createIntersectionObserver,
    createViewportObserver,
    createVisibilityObserver,
    makeIntersectionObserver,
} from "./intersection-observer.ts";

test("observe nothing on the server, reading elements as hidden or as their initial value", () => {
    // observe through accessors that record being read
    const read: string[] = [];
    const [entries, isVisible] = createIntersectionObserver(
        () => {
            read.push("elements");

            return [];
        },
        () => {
            read.push("options");

            return {};
        },
    );
    const element = untouched<Element>();

    expect({
        made: makeIntersectionObserver([], () => {}).instance,
        viewport: createViewportObserver()[1].instance,
        read,
        entries,
        isVisible: isVisible(element),
        visibility: [
            createVisibilityObserver(element)(),
            createVisibilityObserver(element, { initialValue: true })(),
        ],
    }).toEqual({
        made: undefined,
        viewport: undefined,
        read: [],
        entries: [],
        isVisible: false,
        visibility: [false, true],
    });
});
