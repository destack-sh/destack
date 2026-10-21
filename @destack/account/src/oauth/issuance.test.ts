import { AccessError } from "@destack/access";
import { DatabaseError } from "@destack/db";
import { expect, test } from "@destack/test";
import { Issuance } from "./issuance.ts";

test("read a refused grant check as an invalid grant, and pass an outage on as it is", () => {
    const outage = new DatabaseError("CONNECTION_CLOSED", "the database closed");
    const refused = Issuance.grantRefusal(new AccessError("UNAUTHORIZED", "assertion expired"));

    expect([
        refused instanceof Error && "body" in refused ? refused.body : refused,
        Issuance.grantRefusal(outage),
    ]).toEqual([{ error: "invalid_grant", error_description: "assertion expired" }, outage]);
});
