import { ResidencyCode, SpaceKind, StackSource } from "@destack/account/object";
import { OperatorDeclaration } from "@destack/account/server";
import { schema } from "@destack/schema";

/** What a universe declares: its residencies, platform and the stacks its spaces are born with. */
export const UniverseDeclaration = schema.object({
    /** The residencies people choose from. */
    residencies: schema.array(schema.object({ code: ResidencyCode, name: schema.string().min(1) })),
    /** The platform's organisation, its accounts and the cloud machines serving residencies. */
    platform: OperatorDeclaration,
    /** The stack each kind of space is born with. */
    stacks: schema.record(SpaceKind, StackSource),
});
/** What a universe declares. */
export type UniverseDeclaration = schema.Infer<typeof UniverseDeclaration>;
