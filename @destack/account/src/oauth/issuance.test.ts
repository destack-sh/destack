import { AccessError } from "@destack/access";
import { DatabaseError } from "@destack/db";
import { expect, test } from "@destack/test";
import { Issuance } from "./issuance.ts";

test("read a refused grant check as an invalid grant, and pass an outage on as it is", () => {
    const outage = DatabaseError.of("CONNECTION_CLOSED", "the database closed");
    const refused = Issuance.grantRefusal(AccessError.of("ASSERTION_EXPIRED", "assertion expired"));

    expect([refused, Issuance.grantRefusal(outage)]).toEqual([
        Issuance.refusal("invalid_grant", "assertion expired"),
        outage,
    ]);
});
