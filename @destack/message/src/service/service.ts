import { defineService } from "@destack/service";
import { message } from "../object/index.ts";

/** The messages of the spaces a cell serves, sent through each channel's provider. */
export const messageService = defineService("message", { objects: { message } });
