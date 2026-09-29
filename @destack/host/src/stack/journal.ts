import { defineJournal } from "@destack/service/database";

/** The requests the host service executed. */
export const hostJournal = defineJournal("journal");
