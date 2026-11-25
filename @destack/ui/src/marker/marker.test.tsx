import { expect, test } from "@destack/test";
import { markup, render } from "@destack/view/test";
import { Marker, MarkerContent, MarkerIcon } from "./index.ts";

test("render a note and a link of one, the icon left out of what is read", () => {
    const { container } = render(() => (
        <>
            <Marker variant="separator" role="separator">
                <MarkerContent>Today</MarkerContent>
            </Marker>
            <Marker render={(attributes) => <a {...attributes} href="#budget" />}>
                <MarkerIcon>*</MarkerIcon>
                <MarkerContent busy>Reading</MarkerContent>
            </Marker>
        </>
    ));

    expect(markup(container)).toBe(
        [
            '<div data-slot="marker" data-variant="separator" role="separator">',
            '<span data-slot="marker-content">Today</span></div>',
            '<a data-slot="marker" data-variant="default" href="#budget">',
            '<span data-slot="marker-icon" aria-hidden="true">*</span>',
            '<span data-slot="marker-content">Reading</span></a>',
        ].join(""),
    );
});
