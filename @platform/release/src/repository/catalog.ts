import { CHANNELS } from "@destack/update/release";
import { schema } from "@destack/schema";

/** The native installer formats a release publishes beside its update archives. */
export const INSTALLER_FORMATS = ["dmg"] as const;

/** A native installer format. */
export const InstallerFormat = schema.enum(INSTALLER_FORMATS);

/** A native installer format. */
export type InstallerFormat = schema.Infer<typeof InstallerFormat>;

/** One public archive or installer authenticated by targets metadata. */
export const CatalogDownload = schema.object({
    /** Calendar version reported by the installed programs. */
    version: schema.string(),
    /** Immutable HTTPS artifact URL. */
    url: schema.string(),
    /** Lowercase SHA-256 artifact digest. */
    sha256: schema.string(),
    /** Exact archive length in bytes. */
    size: schema.number(),
});

/** One public archive or installer authenticated by targets metadata. */
export type CatalogDownload = schema.Infer<typeof CatalogDownload>;

/** An updater archive and native installer choices for one platform. */
export const CatalogDistribution = schema.object({
    /** Complete archive consumed by the authenticated updater. */
    archive: CatalogDownload.exactOptional(),
    /** Native installers keyed by their file format. */
    installers: schema.partialRecord(InstallerFormat, CatalogDownload),
});

/** An updater archive and native installer choices for one platform. */
export type CatalogDistribution = schema.Infer<typeof CatalogDistribution>;

/** Public download choices derived from one signed release. */
export const Catalog = schema.object({
    /** Calendar version shared by every download. */
    version: schema.string(),
    /** The channel publishing the release. */
    channel: schema.enum(CHANNELS),
    /** Platform-specific archives and installers, by target or installer name. */
    downloads: schema.record(schema.string(), CatalogDistribution),
});

/** Public download choices derived from one signed release. */
export type Catalog = schema.Infer<typeof Catalog>;
