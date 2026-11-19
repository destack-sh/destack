import { expect, test } from "@destack/test";
import { removePages } from "./pages.ts";

test("remove a static view's origin and pages module from its declaration, keeping its shell and client views", () => {
    // declare a static view and a client view
    const code = [
        `import { defineView } from "@destack/view/declare";`,
        `export const site = defineView({`,
        `    name: "site",`,
        `    component: () => import("./app.tsx"),`,
        `    render: {`,
        `        mode: "static",`,
        `        origin: "https://example.test",`,
        `        document: () => import("./document.tsx"),`,
        `        pages: () => import("./pages.ts"),`,
        `    },`,
        `});`,
        `export const notes = defineView({ name: "notes", component: () => import("./notes.tsx"), render: { mode: "client" } });`,
    ].join("\n");

    // keep the static view's mode and shell, and leave the client view as it is
    expect(removePages(code, "view.ts")?.code.split("\n")).toEqual([
        `import { defineView } from "@destack/view/declare";`,
        `export const site = defineView({`,
        `    name: "site",`,
        `    component: () => import("./app.tsx"),`,
        `    render: {`,
        `        mode: "static",`,
        `        `,
        `        document: () => import("./document.tsx"),`,
        `        `,
        `    },`,
        `});`,
        `export const notes = defineView({ name: "notes", component: () => import("./notes.tsx"), render: { mode: "client" } });`,
    ]);
});

test("leave a module without static views unchanged", () => {
    expect(
        removePages(
            `export const notes = defineView({ name: "notes", component: load });`,
            "view.ts",
        ),
    ).toBe(undefined);
});
