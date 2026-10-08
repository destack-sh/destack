import { expect, test } from "@destack/test";
import { draw, markup } from "@destack/view/test";
import { tabsHistoryUnavailable } from "./tabs.example.tsx";

test("connect each tab to its panel and show only the selected panel", () => {
    const frame = draw(tabsHistoryUnavailable).querySelector("[data-slot=example]");
    expect(frame === null ? "" : markup(frame)).toBe(
        '<div data-slot="tabs" data-orientation="horizontal">' +
            '<div role="tablist" data-slot="tabs-list" data-orientation="horizontal" aria-orientation="horizontal" aria-label="Note">' +
            '<button type="button" role="tab" id="id-1-tab-edit" aria-controls="id-1-panel-edit" aria-selected="true" tabindex="0" data-slot="tabs-trigger" data-state="active" data-value="edit" data-orientation="horizontal">Edit</button>' +
            '<button type="button" role="tab" id="id-1-tab-preview" aria-controls="id-1-panel-preview" aria-selected="false" tabindex="-1" data-slot="tabs-trigger" data-state="inactive" data-value="preview" data-orientation="horizontal">Preview</button>' +
            '<button type="button" role="tab" id="id-1-tab-history" aria-controls="id-1-panel-history" aria-selected="false" tabindex="-1" data-slot="tabs-trigger" data-state="inactive" data-value="history" data-orientation="horizontal" disabled="">History</button></div>' +
            '<div role="tabpanel" id="id-1-panel-edit" aria-labelledby="id-1-tab-edit" tabindex="0" data-slot="tabs-content" data-state="active" data-orientation="horizontal">Editor</div>' +
            '<div role="tabpanel" id="id-1-panel-preview" aria-labelledby="id-1-tab-preview" tabindex="0" hidden="" data-slot="tabs-content" data-state="inactive" data-orientation="horizontal">Rendered</div>' +
            '<div role="tabpanel" id="id-1-panel-history" aria-labelledby="id-1-tab-history" tabindex="0" hidden="" data-slot="tabs-content" data-state="inactive" data-orientation="horizontal">Versions</div></div>',
    );
});
