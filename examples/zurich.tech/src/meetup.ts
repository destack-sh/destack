import { z } from "zod";

export enum MeetupStatus {
    Draft = "Draft",
    Upcoming = "Upcoming",
    Cancelled = "Cancelled",
    Past = "Past",
}

export const MeetupStatus1 = z.enum(MeetupStatus);
export type MeetupStatus1 = z.infer<typeof MeetupStatus1>;

export const MeetupLanguage = z.enum(["English", "German"]);
export type MeetupLanguage = z.infer<typeof MeetupLanguage>;

// Represents a series of related Meetups.
export const MeetupSeries = z.object({
    // The name of the Meetup Series.
    name: z.string().describe("The name of the Meetup Series."),
    // The prefix of the Meetup Series.
    prefix: z.string().describe("The prefix of the Meetup Series."),
});
export type MeetupSeries = z.infer<typeof MeetupSeries>;

// Represents a Meetup.
export const Meetup = z.object({
    // When the Meetup is happening.
    date: z.date().describe("When the Meetup is happening."),
    // The name of the Meetup.
    name: z.string().describe("The name of the Meetup."),
    // The number of the Meetup.
    number: z.number().int().nonnegative().describe("The number of the Meetup."),
    // The Series this Meetup is part of.
    seriesId: z.string().describe("The Series this Meetup is part of."),
    // The language of the Meetup.
    language: MeetupLanguage.describe("The language of the Meetup."),
    // The status of the Meetup.
    status: MeetupStatus1.describe("The status of the Meetup."),
});
export type Meetup = z.infer<typeof Meetup>;

// Represents a registration for a Meetup.
export const MeetupRegistration = z.object({
    // The Meetup this registration is for.
    meetupId: z.string().describe("The Meetup this registration is for."),
    // The person registering.
    userId: z.string().describe("The person registering."),
});
export type MeetupRegistration = z.infer<typeof MeetupRegistration>;

// Registers a user for a Meetup.
export function registerForMeetup(meetupId: string, userId: string): MeetupRegistration {
    // create registration object
    const registration = { meetupId, userId };
    return registration;
}
