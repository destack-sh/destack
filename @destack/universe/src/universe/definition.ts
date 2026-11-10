import { CountryCode, ResidencyCode } from "@destack/account/object";
import { PlatformDefinition } from "@destack/account/server";
import { schema } from "@destack/schema";

/** What a universe declares: its residencies and its platform. */
export const UniverseDefinition = schema.object({
    /** The residencies people choose from. */
    residencies: schema.array(
        schema.object({
            code: ResidencyCode,
            name: schema.string().min(1),
            relay: schema.url(),
            countries: schema.array(CountryCode),
        }),
    ),
    /** The platform's organization, its homes, its spaces' stacks and its cloud machines. */
    platform: PlatformDefinition,
});
/** What a universe declares. */
export type UniverseDefinition = schema.Infer<typeof UniverseDefinition>;
