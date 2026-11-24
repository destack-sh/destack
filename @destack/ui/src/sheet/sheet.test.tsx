import { expect, test } from "@destack/test";
import { markup, render } from "@destack/view/test";
import { Sheet, SheetContent, SheetTitle } from "./index.ts";

test("place a sheet against its side", () => {
    const { container } = render(() => (
        <Sheet>
            <SheetContent side="left" showCloseButton={false}>
                <SheetTitle>Filters</SheetTitle>
            </SheetContent>
        </Sheet>
    ));
    expect(markup(container)).toBe(
        '<dialog id="id-1" data-closed="" closedby="any" data-slot="sheet-content" data-side="left" aria-labelledby="id-2"><h2 id="id-2" data-slot="sheet-title">Filters</h2></dialog>',
    );
});
