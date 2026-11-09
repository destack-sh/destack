import { ACCOUNT_TEMPLATE } from "@destack/account/test";
import { BORN_SPACE, ROUTE_TEMPLATE, TestSpace } from "@destack/stack/test";
import { WORK_BOARD, PUBLISHER, TestComputer } from "./computer.ts";

/** The start of January 2026, the month the meter's scenario measures in. */
export const METER_START = Date.UTC(2026, 0, 1);

/** The space of the meter's scenario, from its month's start. */
export const METER_OPTIONS = { start: METER_START };

/** The templates machine's tests copy, which the run writes once. */
export const templates = [
    ACCOUNT_TEMPLATE,
    TestSpace.template({}, BORN_SPACE),
    ...TestComputer.templates(),
    TestComputer.homeTemplate([PUBLISHER]),
    TestComputer.homeTemplate([], WORK_BOARD),
    ROUTE_TEMPLATE,
    TestSpace.template(METER_OPTIONS, BORN_SPACE),
];
