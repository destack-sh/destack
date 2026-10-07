import { schema } from "@destack/schema";

/** The machine every tunnel of the test relay belongs to. */
export const MACHINE = schema
    .identifier("machine")
    .parse("machine-01996ab0-0000-7000-8000-0000000000d1");

/** The machine's tunnel, kept in the EU's jurisdiction. */
export const DESTINATION = { machineId: MACHINE, residencyId: "eu" } as const;

/** The name the test relay admits the machine under. */
export const NAME = "laptop.acme.destack.computer";

/** The path the test relay lists the jurisdictions its machines' objects were reached in at. */
export const JURISDICTION_PATH = "/jurisdictions";

/** The path the test relay tells the machine the name in its query at. */
export const RENAME_PATH = "/rename";
