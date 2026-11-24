import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { render } from "@destack/view/test";
import { SearchField } from "./index.ts";

test("search by Enter and clear by Escape, once", () => {
    const ran: string[] = [];
    const { container } = render(() => (
        <div onKeyDown={(event) => void ran.push(`outside ${event.key}`)}>
            <SearchField
                defaultValue="lake"
                onSubmit={(text) => void ran.push(`search ${text}`)}
                onClear={() => void ran.push("clear")}
            />
        </div>
    ));
    const input = present(container.querySelector("input"), "the input");
    const press = (key: string) => {
        input.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
        flush();
    };
    press("Enter");
    press("Escape");
    const after = input.value;
    press("Escape");

    expect([ran, after, container.querySelector("button")]).toEqual([
        ["search lake", "outside Enter", "clear", "outside Escape"],
        "",
        null,
    ]);
});
