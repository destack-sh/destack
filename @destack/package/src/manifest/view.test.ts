import { expect, test, refusal } from "@destack/test";
import { ViewDescription } from "./view.ts";

/** A view of no type, as an output compiles it. */
const VIEW: ViewDescription = {
    entrypoint: "home.js",
    sandbox: {},
    presents: [],
    categories: [],
    packages: [],
};

test("list the views every output compiles by name, refusing a name two outputs compile", async () => {
    // compile a home and a settings view in two outputs, then the home view in both
    const outputs = (second: string) => ({
        outputs: {
            browser: { views: { home: VIEW } },
            panel: { views: { [second]: { ...VIEW, entrypoint: `${second}.js` } } },
        },
    });

    expect({
        listed: ViewDescription.list(outputs("settings")),
        refused: await refusal(() => ViewDescription.list(outputs("home"))),
    }).toEqual({
        listed: { home: VIEW, settings: { ...VIEW, entrypoint: "settings.js" } },
        refused: [
            "UNPROCESSABLE_CONTENT",
            "VIEW_DUPLICATE",
            "output panel compiles view home, which another output already compiles",
        ],
    });
});
