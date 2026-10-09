import type { Root } from "hast";
import { expect, test } from "@destack/test";
import { Errored } from "@destack/view";
import { render } from "@destack/view/test";
import { Content, type ContentElementProperties } from "./index.ts";

/** A page as remark and rehype build it from Markdown: a heading, a linked footnote reference, a listing and an icon. */
const PAGE: Root = {
    type: "root",
    children: [
        {
            type: "element",
            tagName: "h2",
            properties: { id: "setup" },
            children: [{ type: "text", value: "Setup" }],
        },
        {
            type: "element",
            tagName: "p",
            properties: { className: ["lead", "muted"] },
            children: [
                { type: "text", value: "Install it" },
                {
                    type: "element",
                    tagName: "a",
                    properties: {
                        href: "#fn-1",
                        dataFootnoteRef: true,
                        ariaDescribedBy: ["footnote-label"],
                    },
                    children: [{ type: "text", value: "1" }],
                },
                { type: "comment", value: "draft" },
            ],
        },
        {
            type: "element",
            tagName: "pre",
            properties: { dataTitle: "install.sh" },
            children: [
                {
                    type: "element",
                    tagName: "code",
                    properties: { className: ["language-sh"] },
                    children: [{ type: "text", value: "curl destack.sh/install | sh" }],
                },
            ],
        },
        {
            type: "element",
            tagName: "svg",
            properties: { viewBox: "0 0 24 24", ariaHidden: "true" },
            children: [
                {
                    type: "element",
                    tagName: "rect",
                    properties: { width: 24, height: 2 },
                    children: [],
                },
            ],
        },
    ],
};

/** A listing that frames its code under the title its element carries. */
function Listing(properties: ContentElementProperties) {
    return (
        <figure data-listing={String(properties["data-title"])}>
            <figcaption>{properties.node.tagName}</figcaption>
            {properties.children}
        </figure>
    );
}

test("render a content tree as elements with their HTML and SVG attributes, swapping a mapped tag for its component", () => {
    const { container } = render(() => <Content tree={PAGE} components={{ pre: Listing }} />);

    // name each attribute as HTML or SVG does, leave out the comment, and frame the listing with its component
    expect(container.innerHTML).toBe(
        '<h2 id="setup">Setup</h2>' +
            '<p class="lead muted">Install it<a href="#fn-1" data-footnote-ref="" aria-describedby="footnote-label">1</a></p>' +
            '<figure data-listing="install.sh"><figcaption>pre</figcaption><code class="language-sh">curl destack.sh/install | sh</code></figure>' +
            '<svg viewBox="0 0 24 24" aria-hidden="true"><rect width="24" height="2"></rect></svg>',
    );
});

test("refuse raw HTML a builder left unparsed in the tree", () => {
    const raw: Root = { type: "root", children: [] };
    Reflect.set(raw, "children", [{ type: "raw", value: "<b>bold</b>" }]);
    const { container } = render(() => (
        <Errored fallback={(error) => String(error())}>
            <Content tree={raw} />
        </Errored>
    ));

    // a raw node means the tree was built without parsing its HTML, which the renderer cannot trust
    expect(container.innerHTML).toBe(
        "TypeError: content tree holds raw HTML its builder left unparsed",
    );
});
