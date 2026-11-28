import { expect, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { t } from "@destack/locale";
import { PackageId, PackageVersion } from "@destack/package";
import { Digest, present } from "@destack/schema";
import type { ViewContext } from "../declare/context.ts";
import { defineView } from "../declare/view.ts";
import { ViewServer } from "./server.ts";

/** The package of the served view. */
const tasks = PackageVersion.parse({
    id: PackageId.parse("package-01a0c80b-614e-739e-a21a-66fd749e17ca"),
    name: "@example/tasks",
    version: "2026.11.0",
});

test("prerender a hovered link to another origin of the page's site, as another installation", async () => {
    // serve a page of Tasks
    const view = defineView(
        { name: "tasks", title: t`Tasks`, component: async () => ({ default: () => "Tasks" }) },
        { package: tasks },
    );
    const server = await ViewServer.open(view, {}, testHost());
    const context: ViewContext = {
        installation: "installation-tasks",
        space: "space-home",
        account: "account-ada",
        view: "tasks",
        build: Digest.parse("a".repeat(64)),
        base: "",
        user: { packageId: tasks.id, type: "anonymous", scope: "universe", id: "anonymous" },
        universe: "https://destack.example",
        catalogs: [],
    };
    const response = await server.respond(
        new Request("https://tasks--home--ada.space.example/tasks"),
        { script: "/view.js", stylesheets: [], preloads: [], context },
        null,
    );

    // read the speculation rule the page's head links: links below the page's domain, but its own origin's
    const html = await response.text();
    const rules = /<script type="speculationrules">(.*?)<\/script>/u.exec(html)?.[1];
    expect(JSON.parse(present(rules, "the page's speculation rules"))).toEqual({
        prerender: [
            {
                source: "document",
                where: {
                    and: [
                        { href_matches: "https://*.space.example/*" },
                        { not: { href_matches: "/*" } },
                    ],
                },
                eagerness: "moderate",
            },
        ],
    });
});
