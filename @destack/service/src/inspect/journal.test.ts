import { expect, test } from "@destack/test";
import { defineJournal } from "../database/index.ts";
import { describeJournal } from "./journal.ts";

test("describe a journal by its table's name", () => {
    expect(describeJournal(defineJournal("journal"))).toEqual({ name: "journal" });
});
