import { expect, test } from "@destack/test";
import { classifyError, DatabaseError } from "./error.ts";

test("classify driver failures and leave domain errors that wrap them alone", () => {
    // a driver's unique violation
    const driver = Object.assign(new Error("duplicate key value violates unique constraint"), {
        code: "23505",
    });

    // a domain error explaining the same failure its own way
    const domain = new Error("package version was already published", { cause: driver });

    // classify the driver failure, and keep the domain error as thrown
    const classified = [classifyError(driver), classifyError(domain)];
    expect(
        classified.map((error) => (error instanceof DatabaseError ? error.code : error)),
    ).toEqual(["DUPLICATE", domain]);
});
