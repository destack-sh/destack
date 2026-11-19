import { expect, test } from "@destack/test";
import { writeLlms } from "./llms.ts";

test("write the title, summary and details, then each section's links to absolute Markdown", () => {
    expect(
        writeLlms("https://destack.sh", {
            title: "Destack",
            summary: "The personal software platform.",
            details: "Every page is also Markdown.",
            sections: [
                {
                    title: "Docs",
                    links: [
                        { title: "Setup", path: "/docs/setup.md", description: "Install Destack." },
                    ],
                },
                { title: "Optional", links: [{ title: "Blog", path: "/blog.md" }] },
            ],
        }),
    ).toBe(
        [
            "# Destack",
            "",
            "> The personal software platform.",
            "",
            "Every page is also Markdown.",
            "",
            "## Docs",
            "",
            "- [Setup](https://destack.sh/docs/setup.md): Install Destack.",
            "",
            "## Optional",
            "",
            "- [Blog](https://destack.sh/blog.md)",
            "",
        ].join("\n"),
    );
});
