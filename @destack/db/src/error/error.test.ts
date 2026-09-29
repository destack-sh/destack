import { expect, test } from "@destack/test";
import { DrizzleQueryError } from "drizzle-orm/errors";
import { classifyError, DatabaseError } from "./error.ts";

test("classify driver failures through query wrappers and leave domain errors that wrap them alone", () => {
    // a driver's unique violation, bare and wrapped by a failed query
    const driver = Object.assign(new Error("duplicate key value violates unique constraint"), {
        code: "23505",
    });
    const wrapped = new DrizzleQueryError("insert into note", [], driver);

    // a domain error explaining the same failure its own way
    const domain = new Error("package version was already published", { cause: wrapped });

    // classify the first two, and keep the domain error as thrown
    const classified = [classifyError(driver), classifyError(wrapped), classifyError(domain)];
    expect(
        classified.map((error) => (error instanceof DatabaseError ? error.code : error)),
    ).toEqual(["DUPLICATE", "DUPLICATE", domain]);
});
