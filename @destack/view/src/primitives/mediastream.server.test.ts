import { expect, test } from "@destack/test";
import {
    createAmplitudeFromStream,
    createAmplitudeStream,
    createScreen,
    createStream,
} from "./mediastream.ts";

test("open no streams on the server", () => {
    expect([
        createStream({ audio: true })[0](),
        createScreen({ video: true })[0](),
        createAmplitudeFromStream(undefined)[0](),
        createAmplitudeStream()[0](),
    ]).toEqual([undefined, undefined, 0, 0]);
});
