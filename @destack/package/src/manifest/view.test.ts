import { expect, test } from "@destack/test";
import { ViewDescription } from "./view.ts";

/** A view of no type, as an output compiles it. */
const VIEW: ViewDescription = { entrypoint: "home.js", sandbox: {}, presents: [], packages: [] };

test("list the views every output compiles by name, refusing a name two outputs compile", () => {
    // compile a home and a settings view in two outputs, then the home view in both
    const outputs = (second: string) => ({
        outputs: {
            browser: { views: { home: VIEW } },
            panel: { views: { [second]: { ...VIEW, entrypoint: `${second}.js` } } },
        },
    });

    expect({
        listed: ViewDescription.list(outputs("settings")),
        refused: (() => {
            try {
                return ViewDescription.list(outputs("home"));
            } catch (error) {
                return String(error);
            }
        })(),
    }).toEqual({
        listed: { home: VIEW, settings: { ...VIEW, entrypoint: "settings.js" } },
        refused: "TypeError: output panel compiles view home another output does",
    });
});
