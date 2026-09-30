import { defineJournal } from "@destack/service/database";

/** The space administration requests the space service executed. */
export const spaceJournal = defineJournal("journal");
