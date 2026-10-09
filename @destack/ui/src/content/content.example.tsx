import type { Root } from "hast";
import { defineExample } from "@destack/package/declare";
import { Badge } from "../badge/index.ts";
import { Prose } from "../prose/index.ts";
import { Content, type ContentElementProperties } from "./content.tsx";

/** A release note as remark and rehype build it from Markdown: a heading, a paragraph with inline code, and a strong word. */
const NOTE: Root = {
    type: "root",
    children: [
        {
            type: "element",
            tagName: "h2",
            properties: { id: "sync" },
            children: [{ type: "text", value: "Live sync" }],
        },
        {
            type: "element",
            tagName: "p",
            properties: {},
            children: [
                { type: "text", value: "Every space now syncs its objects as they change, and " },
                {
                    type: "element",
                    tagName: "code",
                    properties: {},
                    children: [{ type: "text", value: "destack build" }],
                },
                { type: "text", value: " reuses unchanged packages. " },
                {
                    type: "element",
                    tagName: "strong",
                    properties: {},
                    children: [{ type: "text", value: "New" }],
                },
            ],
        },
    ],
};

/** Mark strong words as badges, to show a component standing in for a tag. */
function Strong(properties: ContentElementProperties) {
    return <Badge>{properties.children}</Badge>;
}

/** A release note rendered from its tree, its strong words swapped for badges. */
export const contentNote = defineExample({
    of: Content,
    name: "note",
    description: "a release note rendered from its tree, its strong words swapped for badges",
    render: () => (
        <Prose>
            <Content tree={NOTE} components={{ strong: Strong }} />
        </Prose>
    ),
});
