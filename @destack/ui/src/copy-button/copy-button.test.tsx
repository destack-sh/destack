import { expect, onTestFinished, test, vi } from "@destack/test";
import { flush } from "@destack/view";
import { render, stubPopovers, wait } from "@destack/view/test";
import { CopyButton } from "./index.ts";

test("copy the text and say so for a while, or that the browser refused", async () => {
    // take the first text and refuse the second
    stubPopovers();
    const taken: string[] = [];
    vi.stubGlobal("navigator", {
        clipboard: {
            writeText: async (text: string) => {
                if (taken.length > 0) {
                    throw new Error("refused");
                }
                taken.push(text);
            },
        },
    });
    onTestFinished(() => {
        vi.unstubAllGlobals();
    });
    const { container } = render(() => <CopyButton value="link" label="Copy the link" />);
    flush();
    const button = container.querySelector("button");
    const said = () => [
        container.querySelector("[role=status]")?.textContent,
        button?.getAttribute("data-outcome"),
        container.querySelector("[data-slot=tooltip-content]")?.textContent,
    ];
    const press = async () => {
        button?.click();
        await wait(0);
        flush();

        return said();
    };
    const copied = await press();
    await wait(3000);
    flush();
    const later = said();
    const refused = await press();

    expect({
        copied,
        later,
        refused,
        taken,
        label: button?.getAttribute("aria-label"),
    }).toEqual({
        copied: ["Copied", "copied", "Copied"],
        later: ["", "none", undefined],
        refused: ["Not copied", "refused", "Not copied"],
        taken: ["link"],
        label: "Copy the link",
    });
});
