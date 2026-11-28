import { expect, test } from "@destack/test";
import { t } from "@destack/locale";
import { ViewApp } from "./app.ts";

test("open an app's main view at its installation origin beside the page's, on the page's port", () => {
    // write a task app of Ada's home space from a page of another installation of it
    const app: ViewApp = {
        installation: "installation-01996ab0-0000-7000-8000-0000000000e1",
        name: "tasks",
        view: "tasks",
        title: t`Tasks`,
        icon: "check-square",
        color: "blue",
        isHome: false,
    };

    expect([
        ViewApp.href(
            app,
            "home--ada",
            new URL("http://notes--home--ada.space.localhost:4100/notes"),
        ),
        ViewApp.href(app, "home--ada", new URL("https://home--ada.space.example.com/")),
    ]).toEqual([
        "http://tasks--home--ada.space.localhost:4100/tasks",
        "https://tasks--home--ada.space.example.com/tasks",
    ]);
});
