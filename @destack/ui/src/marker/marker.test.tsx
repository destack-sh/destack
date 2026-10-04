import { expect, test } from "@destack/test";
import { draw, markup } from "@destack/view/test";
import { Marker, MarkerContent, MarkerIcon } from "./index.ts";

test("render a marker with a hidden icon beside its text", () => {
    const container = draw(() => (
        <Marker variant="border">
            <MarkerIcon>+</MarkerIcon>
            <MarkerContent>Grace joined</MarkerContent>
        </Marker>
    ));
    expect(markup(container)).toBe(
        '<div data-slot="marker" data-variant="border">' +
            '<span data-slot="marker-icon" aria-hidden="true">+</span>' +
            '<span data-slot="marker-content">Grace joined</span></div>',
    );
});
