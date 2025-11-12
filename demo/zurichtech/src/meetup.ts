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
    name: z.string(),
    // The prefix of the Meetup Series.
    prefix: z.string(),
});
export type MeetupSeries = z.infer<typeof MeetupSeries>;

// Represents a Meetup.
export const Meetup = z.object({
    // When the Meetup is happening.
    date: z.date(),
    // The name of the Meetup.
    name: z.string(),
    // The number of the Meetup.
    number: z.number().int().nonnegative(),
    // The Series this Meetup is part of.
    seriesId: z.string(),
    // The language of the Meetup.
    language: MeetupLanguage,
    // The status of the Meetup.
    status: MeetupStatus,
});
export type Meetup = z.infer<typeof Meetup>;

// Represents a registration for a Meetup.
export const MeetupRegistration = z.object({
    // The Meetup this registration is for.
    meetupId: z.string(),
    // The person registering.
    userId: z.string(),
});
export type MeetupRegistration = z.infer<typeof MeetupRegistration>;

// Registers a user for a Meetup.
export function registerForMeetup(meetupId: string, userId: string): MeetupRegistration {
    // create registration object
    const registration = { meetupId, userId };
    return registration;
}
