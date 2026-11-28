import { expect, test } from "@destack/test";
import { t } from "@destack/locale";
import { Digest } from "@destack/schema";
import { PackageId } from "@destack/package";
import type { ViewContext } from "../declare/context.ts";
import { PageServer } from "./serve.ts";

test("prerender the space's other apps on hover, at their main views beside the page", () => {
    // render a page of Tasks in a space with Home and Tasks
    const context: ViewContext = {
        installation: "installation-tasks",
        space: "space-home",
        spaceAddress: "home--ada",
        spaceTitle: "Home",
        apps: [
            {
                installation: "installation-home",
                name: "home",
                view: "start",
                title: t`Home`,
                icon: "house",
                color: "gray",
                isHome: true,
            },
            {
                installation: "installation-tasks",
                name: "tasks",
                view: "tasks",
                title: t`Tasks`,
                icon: "check-square",
                color: "blue",
                isHome: false,
            },
        ],
        account: "account-ada",
        view: "tasks",
        build: Digest.parse("a".repeat(64)),
        base: "",
        user: {
            packageId: PackageId.parse("package-01a0c80b-614e-739e-a21a-66fd749e17ca"),
            type: "anonymous",
            scope: "universe",
            id: "anonymous",
        },
        universe: "https://destack.example",
        catalogs: [],
    };
    const page = new URL("https://tasks--home--ada.space.example/tasks?task=a");

    expect(PageServer.speculation(context, page)).toEqual(
        '<script type="speculationrules">{"prerender":[{"urls":["https://ada.space.example/start"],"eagerness":"moderate"}]}</script>',
    );
});
