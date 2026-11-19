import { present } from "@destack/schema";
import { artifactMarkOf, type Publisher, publishers } from "../command/artifact";
import { type Assistant, assistantOptions } from "../command/chat";
import type { Option, OptionSection } from "../figure/picker";

/** Where an agent leaves its app: an assistant's artifact, or a sandbox at its preview address. */
export type Place = { kind: "artifact"; assistant: Publisher } | { kind: "sandbox"; host: string };

/** An agent an app is made with: its name, mark, agent and where it leaves the app. */
export type Maker = Option & { agent: Assistant; place: Place };

/** Every agent an app is made with: the assistants that publish artifacts, then Amp's sandbox. */
export const makers: readonly Maker[] = [
    ...publishers.map((publisher): Maker => ({
        name: publisher,
        mark: artifactMarkOf(publisher),
        agent: publisher,
        place: { kind: "artifact", assistant: publisher },
    })),
    {
        name: "Amp",
        mark: present(
            assistantOptions.find((option) => option.name === "Amp"),
            "Amp's mark",
        ).mark,
        agent: "Amp",
        place: { kind: "sandbox", host: "launch-tracker.onamp.dev" },
    },
];

/** The picker's headings: each kind of place, and the agents that leave an app there. */
export const makerSections: readonly OptionSection[] = (
    [
        ["Artifact", "artifact"],
        ["Sandbox", "sandbox"],
    ] as const
).map(([label, kind]): OptionSection => [
    label,
    makers.filter((maker) => maker.place.kind === kind).map((maker) => maker.name),
]);

/** Return the maker a picker option names, refusing any other name. */
export function makerOf(name: string): Maker {
    const maker = makers.find((candidate) => candidate.name === name);
    if (maker === undefined) {
        throw new RangeError(`maker '${name}' not found`);
    }

    return maker;
}
