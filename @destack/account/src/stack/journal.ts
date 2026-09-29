import { defineJournal } from "@destack/service/database";

/** The administrative requests the account service executed. */
export const accountJournal = defineJournal("journal");
