import { refusal } from "@destack/test";

/** Report a call's outcome as its failure code and message, or as executed. */
export async function outcome(call: Promise<unknown>): Promise<string> {
    const refused = await refusal(call);

    return refused === "done" ? "executed" : refused.join(": ");
}
