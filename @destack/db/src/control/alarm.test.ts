import { expect, test } from "@destack/test";
import { AlarmClock, AlarmStorage } from "./alarm.ts";

test("set one host alarm at the earliest time any party sharing it needs, and clear it once none does", async () => {
    const alarms: (number | null)[] = [];
    const clock = new AlarmClock(
        {
            setAlarm: async (at) => {
                alarms.push(at);
            },
            deleteAlarm: async () => {
                alarms.push(null);
            },
        },
        AlarmStorage.memory(),
    );
    const [first, second] = [clock.alarm("first"), clock.alarm("second")];

    // move the alarm to the earliest need
    await first.setAlarm(100);
    await second.setAlarm(50);
    await second.setAlarm(70);
    await first.setAlarm(80);
    await second.deleteAlarm();
    await first.deleteAlarm();

    expect(alarms).toEqual([100, 50, 70, 80, null]);
});

test("wake the parties due at a ring, keep a wake-up a party sets while woken, and set the host's alarm again", async () => {
    // keep the wake-ups in a storage that outlives the clock, as a Durable Object's does
    const stored = new Map<string, unknown>();
    const alarms: (number | null)[] = [];
    const clock = new AlarmClock(
        {
            setAlarm: async (at) => {
                alarms.push(at);
            },
            deleteAlarm: async () => {
                alarms.push(null);
            },
        },
        {
            put: async (key, value) => {
                stored.set(key, value);
            },
            delete: async (key) => stored.delete(key),
            list: async ({ prefix }) =>
                new Map([...stored].filter(([key]) => key.startsWith(prefix))),
        },
    );
    const [space, notes, chat] = [clock.alarm("space"), clock.alarm("notes"), clock.alarm("chat")];

    // set three wake-ups, the earliest arming the host
    await space.setAlarm(300);
    await notes.setAlarm(100);
    await chat.setAlarm(200);

    // ring at 200: wake notes and chat, chat asking again at 400, then arm the earliest left
    const woken: string[][] = [];
    await clock.fire(200, async (parties) => {
        woken.push([...parties]);
        await chat.setAlarm(400);
    });

    // clear the space's wake-up, leaving chat's
    await space.deleteAlarm();

    expect([woken, alarms, [...stored]]).toEqual([
        [["notes", "chat"]],
        [300, 100, 300, 400],
        [["wakeup:chat", 400]],
    ]);
});
