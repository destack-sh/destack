import { expect, test, refusal } from "@destack/test";
import { ViewDescription } from "./view.ts";

/** A view of no type, as an output compiles it. */
const VIEW: ViewDescription = {
    entrypoint: "home.js",
    stylesheets: [],
    sandbox: {},
    presents: [],
    categories: [],
    audience: "person",
    packages: [],
    render: { mode: "client" },
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
            "BAD_REQUEST",
            "VIEW_INVALID",
            "view 'home' is invalid: output 'panel' compiles it, and another output already does",
        ],
    });
});

test("find the main view the package's origin opens at its root, none without one, refusing two", async () => {
    // mark one view main, then none, then two
    const main = { ...VIEW, categories: ["main" as const, "home" as const] };
    const settings = { ...VIEW, entrypoint: "settings.js" };

    expect({
        found: ViewDescription.main({ home: main, settings }),
        none: ViewDescription.main({ home: VIEW, settings }),
        refused: await refusal(() => ViewDescription.main({ home: main, settings: main })),
    }).toEqual({
        found: "home",
        none: undefined,
        refused: [
            "BAD_REQUEST",
            "VIEW_MAIN_INVALID",
            "main view is invalid: views 'home, settings' are each main",
        ],
    });
});
