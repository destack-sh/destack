import type { Person } from "@destack/account/client";
import { Failure } from "@destack/error";
import { GrantRefusal } from "@destack/object/error";
import type { Prompt } from "../output/index.ts";

/** A terminal's calls, each sent again once its person grants what a refusal asks. */
export class TerminalAuthorization {
    /** The person the terminal acts as, who grants delegates. */
    readonly #person: Person;
    /** The prompt the person answers. */
    readonly #prompt: Prompt;

    /** Authorize as a person, asking them at a prompt. */
    constructor(person: Person, prompt: Prompt) {
        this.#person = person;
        this.#prompt = prompt;
    }

    /** Wrap a send so a call a delegate's grant lacks is asked, granted and sent again. */
    fetch(send: (request: Request) => Promise<Response>): (request: Request) => Promise<Response> {
        return async (request) => {
            // send the call, keeping its body to send it again
            const body = await request.arrayBuffer();
            const copy = () =>
                new Request(request.url, {
                    method: request.method,
                    headers: request.headers,
                    ...(body.byteLength === 0 ? {} : { body }),
                });
            const response = await send(copy());
            const refusal = response.ok ? undefined : await Failure.read(response.clone());
            const asked =
                refusal?.reason === "GRANT_INSUFFICIENT"
                    ? GrantRefusal.safeParse(refusal.data)
                    : undefined;
            const delegate = asked?.success === true ? asked.data.delegate : undefined;
            if (asked?.success !== true || delegate === undefined) {
                return response;
            }

            // ask the person, leaving the refusal when they decline
            const { permission, object } = asked.data;
            const question = `Allow ${delegate.id} to ${permission.name} ${permission.type} ${object.id}?`;
            if (!(await this.#prompt.confirm(question))) {
                return response;
            }

            // grow their grant to the delegate by the permission on the object, then send the call again
            await this.#person.grant(delegate, [{ ...permission, objectId: object.id }]);

            return await send(copy());
        };
    }
}
