import { type ColumnValue, Expression, TABLE } from "@destack/db";
import { found } from "@destack/schema";
import { expect, test } from "@destack/test";
import { computer, MachineEnrollment } from "./computer.ts";

test("convert enrollments stored before 2026.11.0 from their issuer to their universe, keeping an absent one", () => {
    // convert a personal, a cloud and an absent enrollment as their stored rows had them
    const conversion = found(new Map(Object.entries(computer.table[TABLE].convert)), "2026.11.0");
    const expressions = Object.entries(conversion).flatMap(([name, expression]) =>
        expression === undefined ? [] : [[name, expression] as const],
    );
    const expression = found(new Map(expressions), "enrollment");
    const convert = (enrollment: ColumnValue) => Expression.evaluate(expression, { enrollment });
    const personal = {
        kind: "personal",
        issuer: "https://destack.app",
        accountId: "account-01996ab0-0000-7000-8000-000000000001",
        status: "enrolled",
    };
    const cloud = {
        kind: "cloud",
        issuer: "https://destack.app",
        residencyId: "eu",
        status: "revoked",
    };
    const converted = [convert(personal), convert(cloud), convert(null)];

    // read each as a current enrollment, with its universe where its issuer was
    expect([
        MachineEnrollment.parse(converted[0]),
        MachineEnrollment.parse(converted[1]),
        converted[2],
    ]).toEqual([
        {
            kind: "personal",
            universe: "https://destack.app",
            accountId: personal.accountId,
            status: "enrolled",
        },
        { kind: "cloud", universe: "https://destack.app", residencyId: "eu", status: "revoked" },
        null,
    ]);
});
