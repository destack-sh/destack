import type { ServiceErrorReport } from "@destack/schema";
import { expect, test } from "@destack/test";
import { classifyError, DatabaseError, type DatabaseErrorCode } from "./error.ts";

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

test("map every database failure code to the service error its caller receives", () => {
    // convert a failure of each code
    const codes: readonly DatabaseErrorCode[] = [
        "INVALID_MIGRATION",
        "MIGRATION_FAILED",
        "PLAN_CHANGED",
        "NOT_APPLIED",
        "CONNECTION_CLOSED",
        "OWNER_CHANGED",
        "TRANSACTION_CLOSED",
        "TRANSACTION_REQUIRED",
        "TREE_NOT_FOUND",
        "CHANGES_COMPACTED",
        "STALE_EPOCH",
        "CONCURRENT_UPDATE",
        "DUPLICATE",
        "BROKEN_REFERENCE",
        "INVALID_RECORD",
        "INVALID_QUERY",
        "QUERY_FAILED",
        "INVALID_BLOB",
        "NO_CHANNEL",
    ];
    const received = Object.fromEntries(
        codes.map((code) => [code, new DatabaseError(code, "failed").toServiceError()]),
    );

    // receive each code's service error with the failure's message
    const expected: Readonly<Record<DatabaseErrorCode, ServiceErrorReport>> = {
        INVALID_MIGRATION: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
        MIGRATION_FAILED: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
        PLAN_CHANGED: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
        NOT_APPLIED: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
        CONNECTION_CLOSED: { code: "SERVICE_UNAVAILABLE", message: "failed" },
        OWNER_CHANGED: { code: "SERVICE_UNAVAILABLE", message: "failed" },
        TRANSACTION_CLOSED: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
        TRANSACTION_REQUIRED: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
        TREE_NOT_FOUND: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
        CHANGES_COMPACTED: { code: "GONE", message: "failed" },
        STALE_EPOCH: { code: "STALE_EPOCH", message: "failed" },
        CONCURRENT_UPDATE: { code: "CONFLICT", message: "failed" },
        DUPLICATE: { code: "CONFLICT", message: "failed" },
        BROKEN_REFERENCE: { code: "CONFLICT", message: "failed" },
        INVALID_RECORD: { code: "BAD_REQUEST", message: "failed" },
        INVALID_QUERY: { code: "BAD_REQUEST", message: "failed" },
        QUERY_FAILED: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
        INVALID_BLOB: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
        NO_CHANNEL: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
    };
    expect(received).toEqual(expected);
});
