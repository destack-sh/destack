import { principal, relation } from "@destack/access";
import { defineObject, field, method } from "@destack/object";
import { schema } from "@destack/schema";
import { space } from "@destack/space/object";

/** The recipients of the postcards sent, in order. */
export const sent: string[] = [];

/** A postcard, sent as external work once its creation commits. */
export const postcard = defineObject({
    name: "postcard",
    plural: "postcards",
    scope: space,
    fields: {
        /** The person who sent the postcard. */
        sender: field.reference(principal.user).caller(),
        /** The recipient's name. */
        to: field.string(schema.string().min(1)),
    },
    permissions: {
        read: relation("sender"),
        send: relation("sender"),
    },
    methods: {
        list: method.list("read"),
        create: method.create("send", { fields: ["to"], isPredicted: false }).handle({
            prepare: async (call) => call.input.to,
            settle: async (_call, prepared, isCommitted) => {
                if (isCommitted) {
                    sent.push(String(prepared));
                }
            },
        }),
    },
});
