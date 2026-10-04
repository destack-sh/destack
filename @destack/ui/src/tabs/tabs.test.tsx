import { expect, test } from "@destack/test";
import { flush } from "solid-js";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./index.ts";
import { draw, focused, markup, press } from "@destack/view/test";

/** Render tabs for a note's views. */
function drawTabs(activationMode: "automatic" | "manual", changes: string[]): HTMLElement {
    return draw(() => (
        <Tabs
            defaultValue="edit"
            activationMode={activationMode}
            onValueChange={(value) => changes.push(value)}
        >
            <TabsList aria-label="Note">
                <TabsTrigger value="edit">Edit</TabsTrigger>
                <TabsTrigger value="preview">Preview</TabsTrigger>
                <TabsTrigger value="history" disabled>
                    History
                </TabsTrigger>
            </TabsList>
            <TabsContent value="edit">Editor</TabsContent>
            <TabsContent value="preview">Rendered</TabsContent>
            <TabsContent value="history">Versions</TabsContent>
        </Tabs>
    ));
}

test("connect each tab to its panel and show only the selected panel", () => {
    const container = drawTabs("automatic", []);
    expect(markup(container)).toBe(
        '<div data-slot="tabs" data-orientation="horizontal">' +
            '<div role="tablist" data-slot="tabs-list" aria-orientation="horizontal" aria-label="Note">' +
            '<button type="button" role="tab" id="id-1-tab-edit" aria-controls="id-1-panel-edit" aria-selected="true" tabindex="0" data-slot="tabs-trigger" data-state="active" data-value="edit">Edit</button>' +
            '<button type="button" role="tab" id="id-1-tab-preview" aria-controls="id-1-panel-preview" aria-selected="false" tabindex="-1" data-slot="tabs-trigger" data-state="inactive" data-value="preview">Preview</button>' +
            '<button type="button" role="tab" id="id-1-tab-history" aria-controls="id-1-panel-history" aria-selected="false" tabindex="-1" data-slot="tabs-trigger" data-state="inactive" data-value="history" disabled="">History</button></div>' +
            '<div role="tabpanel" id="id-1-panel-edit" aria-labelledby="id-1-tab-edit" tabindex="0" data-slot="tabs-content">Editor</div>' +
            '<div role="tabpanel" id="id-1-panel-preview" aria-labelledby="id-1-tab-preview" tabindex="0" hidden="" data-slot="tabs-content">Rendered</div>' +
            '<div role="tabpanel" id="id-1-panel-history" aria-labelledby="id-1-tab-history" tabindex="0" hidden="" data-slot="tabs-content">Versions</div></div>',
    );
});

test("select tabs as the arrow keys move the focus, skipping disabled tabs and wrapping", () => {
    const changes: string[] = [];
    const container = drawTabs("automatic", changes);
    container.querySelector<HTMLElement>("[role=tab]")?.focus();
    const visited = ["ArrowRight", "ArrowRight", "ArrowLeft", "End", "Home"].map((key) => {
        press(key);
        flush();

        return focused();
    });
    expect([visited, changes]).toEqual([
        ["Preview", "Edit", "Preview", "Preview", "Edit"],
        ["preview", "edit", "preview", "preview", "edit"],
    ]);
});

test("move the focus without selecting in manual activation, selecting on click", () => {
    const changes: string[] = [];
    const container = drawTabs("manual", changes);
    container.querySelector<HTMLElement>("[role=tab]")?.focus();
    press("ArrowRight");
    flush();
    const moved = [focused(), changes.length];
    container.querySelectorAll<HTMLElement>("[role=tab]")[1]?.click();
    flush();
    expect([
        moved,
        changes,
        container.querySelector("[role=tabpanel]:not([hidden])")?.textContent,
    ]).toEqual([["Preview", 0], ["preview"], "Rendered"]);
});
