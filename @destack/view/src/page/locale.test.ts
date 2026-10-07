import { expect, test } from "@destack/test";
import { children, createComponent, createRoot } from "solid-js";
import { Localization, t } from "@destack/locale";
import { LocaleContext, useLocale } from "./locale.ts";

/** The first strong isolate MessageFormat 2 places around a string value. */
const OPEN = String.fromCodePoint(0x2068);

/** The pop directional isolate closing a string value. */
const CLOSE = String.fromCodePoint(0x2069);

/** Read the locale of the current tree and one message rendered in it. */
function status(): string {
    const locale = useLocale();

    return `${locale.tag}: ${locale.render(t`Saved ${"3"} notes`)}`;
}

test("render messages in the source language outside a provider, and in a locale a page provides", () => {
    // read the status alone and inside a provided German locale
    let provided = "";
    const alone = createRoot((dispose) => {
        const text = status();
        dispose();

        return text;
    });
    createRoot((dispose) => {
        // read the provider's lazy children, which run inside the provided locale
        const tree = children(() =>
            createComponent(LocaleContext, {
                value: Localization.of("de", []),
                get children() {
                    provided = status();

                    return undefined;
                },
            }),
        );
        tree();
        dispose();
    });

    expect([alone, provided]).toEqual([
        `en: Saved ${OPEN}3${CLOSE} notes`,
        `de: Saved ${OPEN}3${CLOSE} notes`,
    ]);
});
