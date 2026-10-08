import { telemetry } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";

/** Record a handled failure of the site as an exception. */
export const { captureException } = telemetry.scope(import.meta.destack.package);
