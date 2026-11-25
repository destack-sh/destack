import { RegionDefinition } from "@destack/account/object";
import { PlatformDefinition } from "@destack/account/server";
import { schema } from "@destack/schema";

/** What a universe declares: its regions and its platform. */
export const UniverseDefinition = schema.object({
    /** The regions people choose from. */
    regions: schema.array(RegionDefinition),
    /** The platform's organization, its homes, its spaces' stacks and its managed machines. */
    platform: PlatformDefinition,
});
/** What a universe declares. */
export type UniverseDefinition = schema.Infer<typeof UniverseDefinition>;
