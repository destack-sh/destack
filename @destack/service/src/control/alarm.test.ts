import { expect, test } from "@destack/test";
import { AlarmClock } from "./alarm.ts";

test("keep one host alarm at the earliest time any loop sharing it needs, and clear it once none does", async () => {
    const alarms: (number | null)[] = [];
    const clock = new AlarmClock({
        setAlarm: async (at) => {
            alarms.push(at);
        },
        deleteAlarm: async () => {
            alarms.push(null);
        },
    });
    const [first, second] = [clock.alarm(), clock.alarm()];

    // move the alarm to the earliest need, leaving it for a later one
    await first.setAlarm(100);
    await second.setAlarm(50);
    await second.setAlarm(70);
    await first.setAlarm(80);
    await second.deleteAlarm();
    await first.deleteAlarm();

    expect(alarms).toEqual([100, 50, 70, 80, null]);
});
