import { expect, onTestFinished, test } from "@destack/test";
import { render } from "@solidjs/web";
import { Script } from "./script.tsx";

/** Render scripts into a page, removing what they added after the test. */
async function mount(element: () => ReturnType<typeof Script>): Promise<void> {
    const host = document.createElement("main");
    document.body.append(host);
    const dispose = render(element, host);
    onTestFinished(() => {
        dispose();
        host.remove();
        document.head.replaceChildren();
        for (const script of document.body.querySelectorAll("script")) {
            script.remove();
        }
    });

    // wait for the head registry and the settled page
    await new Promise((resolve) => {
        setTimeout(resolve, 0);
    });
}

test("load a script once after the page hydrates, however many components ask for it", async () => {
    // ask for the same analytics script twice
    await mount(() => (
        <>
            <Script src="data:text/javascript,0" />
            <Script src="data:text/javascript,0" strategy="afterInteractive" />
        </>
    ));

    // add one asynchronous script to the body
    expect(
        [...document.body.querySelectorAll("script")].map((script) => [script.src, script.async]),
    ).toEqual([["data:text/javascript,0", true]]);
});

test("write a script that loads before hydration into the head", async () => {
    // ask for a consent script before the page hydrates
    await mount(() => <Script src="data:text/javascript,1" strategy="beforeInteractive" />);

    // render it in the head, not the body
    expect([
        document.head.querySelector("script")?.getAttribute("src"),
        document.body.querySelectorAll("script").length,
    ]).toEqual(["data:text/javascript,1", 0]);
});
