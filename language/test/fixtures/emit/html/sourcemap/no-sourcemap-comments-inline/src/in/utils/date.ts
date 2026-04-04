import { padZero } from "./numbers.ts";

export const formatDate = (date) =>
    `${date.getFullYear()}-${padZero(date.getMonth() + 1)}-${padZero(date.getDate())}`;
