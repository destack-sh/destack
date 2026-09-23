import { Subject } from "@destack/access";
import { PackageId } from "@destack/package";
import { defineSchema, identifier, schema } from "@destack/schema";

/** The complete identity of a person whose settings are selected. */
export const SettingUser = defineSchema(Subject.extend({ kind: schema.literal("user") }));
/** An authority-qualified person selected for personal settings. */
export type SettingUser = schema.Infer<typeof SettingUser>;

/** A space or an installation within that space. */
export const SettingLocation = defineSchema(
    schema.object({
        /** The containing space. */
        spaceId: identifier("space"),
        /** The receiving installation, when the selection is installation-specific. */
        installationId: identifier("installation").optional(),
    }),
);

/** A typed selection of the person or shared runtime being configured. */
export const SettingTarget = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** Personal configuration for an authority-qualified user. */
            kind: schema.literal("user"),
            /** The person represented by the authenticated caller. */
            user: SettingUser,
            /** The consuming package, independently of the package declaring the setting. */
            packageId: PackageId.optional(),
            /** An optional space or installation refinement. */
            location: SettingLocation.optional(),
            /** An optional authenticated device refinement. */
            deviceId: identifier("device").optional(),
        }),
        schema.object({
            /** Shared configuration independent of the interactive caller. */
            kind: schema.literal("space"),
            /** The configured space or installation. */
            location: SettingLocation,
        }),
        schema.object({
            /** Configuration local to one execution host. */
            kind: schema.literal("host"),
            /** The configured host. */
            hostId: identifier("host"),
        }),
    ]),
);
/** A person or runtime selection, verified before resolution. */
export type SettingTarget = schema.Infer<typeof SettingTarget>;

/** A verified resolution target, including an anonymous visitor without personal assignments. */
export const SettingSelection = defineSchema(
    schema.union([
        SettingTarget,
        schema.object({
            /** Personal presentation defaults for an anonymous visitor. */
            kind: schema.literal("user"),
            /** Anonymous callers have no persistent personal assignment identity. */
            user: schema.null(),
            /** The consuming package selected by the host. */
            packageId: PackageId.optional(),
            /** The receiving space or installation. */
            location: SettingLocation.optional(),
        }),
    ]),
);
/** The host-selected resolution context. */
export type SettingSelection = schema.Infer<typeof SettingSelection>;

/** The administrative scope whose rules apply to a setting. */
export const SettingAuthority = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** Account administration, limited to its verified scope. */
            kind: schema.literal("account"),
            /** The administering account. */
            accountId: identifier("account"),
        }),
        schema.object({
            /** Space administration. */
            kind: schema.literal("space"),
            /** The administering space. */
            spaceId: identifier("space"),
        }),
        schema.object({
            /** Local host administration. */
            kind: schema.literal("host"),
            /** The administering host. */
            hostId: identifier("host"),
        }),
    ]),
);
/** An authority qualified independently of assignment targets. */
export type SettingAuthority = schema.Infer<typeof SettingAuthority>;
