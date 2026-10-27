import { artifactMarkOf } from "../command/artifact";
import { type Assistant, assistants, type Look } from "../command/chat";
import type { Option, OptionSection } from "../figure/picker";

/** Where an agent leaves its app: an assistant's artifact, your dev server, or a platform. */
export type Place =
    | { kind: "artifact"; assistant: Assistant }
    | { kind: "localhost"; chat: "terminal" | "builder" }
    | { kind: "platform"; host: string };

/** An agent an app is made with: its name and mark, and where it leaves the app. */
export type Maker = Option & { place: Place };

/** Every agent an app is made with: assistants, then coding agents, then platforms. */
export const makers: readonly Maker[] = [
    ...assistants.map((assistant): Maker => ({
        name: assistant,
        mark: artifactMarkOf(assistant),
        place: { kind: "artifact", assistant },
    })),
    {
        name: "Claude Code",
        mark: { icon: "claude", tint: "#c96442" },
        place: { kind: "localhost", chat: "terminal" },
    },
    {
        name: "Codex",
        mark: { icon: "openai", tint: "#000000" },
        place: { kind: "localhost", chat: "terminal" },
    },
    {
        name: "Cursor",
        mark: { icon: "cursor", tint: "#000000" },
        place: { kind: "localhost", chat: "builder" },
    },
    {
        name: "Lovable",
        mark: { icon: "lovable", tint: "#ff4f6d" },
        place: { kind: "platform", host: "launch-tracker.lovable.app" },
    },
    {
        name: "v0",
        mark: { icon: "v0", tint: "#000000" },
        place: { kind: "platform", host: "launch-tracker.vercel.app" },
    },
    {
        name: "Bolt",
        mark: { icon: "stackblitz", tint: "#1389fd" },
        place: { kind: "platform", host: "launch-tracker.bolt.host" },
    },
    {
        name: "Replit",
        mark: { icon: "replit", tint: "#f26207" },
        place: { kind: "platform", host: "launch-tracker.replit.app" },
    },
];

/** The picker's headings: each kind of place, and the agents that leave an app there. */
export const makerSections: readonly OptionSection[] = (
    [
        ["Artifact", "artifact"],
        ["Localhost", "localhost"],
        ["Platform", "platform"],
    ] as const
).map(([label, kind]): OptionSection => [
    label,
    makers.filter((maker) => maker.place.kind === kind).map((maker) => maker.name),
]);

/** Return the look of a maker's chat: its assistant's app, a terminal, or a builder's panel. */
export function lookOf(maker: Maker): Look {
    const place = maker.place;
    if (place.kind === "artifact") {
        return { kind: "assistant", assistant: place.assistant };
    } else if (place.kind === "localhost") {
        return { kind: place.chat, name: maker.name, mark: maker.mark };
    }

    return { kind: "builder", name: maker.name, mark: maker.mark };
}

/** Return the maker a picker option names, refusing any other name. */
export function makerOf(name: string): Maker {
    const maker = makers.find((candidate) => candidate.name === name);
    if (maker === undefined) {
        throw new RangeError(`no maker named ${name}`);
    }

    return maker;
}
