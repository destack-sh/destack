import { expect } from "@destack/test";

/** Match any instance of a type, such as a time or digest a test cannot know. */
export function any(type: abstract new (...input: never[]) => unknown): unknown {
    return expect.any(type);
}
