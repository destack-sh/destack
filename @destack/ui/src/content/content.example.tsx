import { t } from "@destack/locale";
import type { Root } from "hast";
import { defineExample } from "@destack/package/declare";
import { Badge } from "../badge/index.ts";
import { Prose } from "../prose/index.ts";
import { Content, type ContentElementProperties } from "./content.tsx";

/** A release note as remark and rehype build it from Markdown. */
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
    description: t`A release note rendered from its tree, its strong words swapped for badges.`,
    render: () => (
        <Prose>
            <Content tree={NOTE} components={{ strong: Strong }} />
        </Prose>
    ),
});

/** The steps of a setup guide as remark and rehype build them from Markdown. */
const STEPS: Root = {
    type: "root",
    children: [
        {
            type: "element",
            tagName: "h2",
            properties: { id: "setup" },
            children: [{ type: "text", value: "Set up a space" }],
        },
        {
            type: "element",
            tagName: "ol",
            properties: {},
            children: ["Create the space.", "Invite your team.", "Install the first app."].map(
                (step) => ({
                    type: "element",
                    tagName: "li",
                    properties: {},
                    children: [{ type: "text", value: step }],
                }),
            ),
        },
    ],
};

/** A setup guide rendered from its tree with the elements the tree has. */
export const contentSetupSteps = defineExample({
    of: Content,
    name: "setupSteps",
    description: t`A setup guide rendered from its tree with the elements the tree has.`,
    render: () => (
        <Prose>
            <Content tree={STEPS} />
        </Prose>
    ),
});
