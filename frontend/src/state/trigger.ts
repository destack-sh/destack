import { useTimeFromNow } from "@/composables/useNow";
import { ScheduleType, TriggerType, type Trigger } from "@/gql/graphql";
import {
  PlayCircleIcon as PlayCircleIconOutline,
  LinkIcon as LinkIconOutline,
  PencilSquareIcon as PencilSquareIconOutline,
  EnvelopeIcon as EnvelopeIconOutline,
  ClockIcon as ClockIconOutline,
  UserCircleIcon as UserCircleIconOutline,
} from "@heroicons/vue/24/outline";
import {
  LinkIcon as LinkIconSolid,
  PencilSquareIcon as PencilSquareIconSolid,
  PlayCircleIcon as PlayCircleIconSolid,
  EnvelopeIcon as EnvelopeIconSolid,
  ClockIcon as ClockIconSolid,
  UserCircleIcon as UserCircleIconSolid,
} from "@heroicons/vue/24/solid";
import { DateTime } from "luxon";
import { ref, watch, type Ref } from "vue";
import cronInterval from "cron-parser";
import cronHunaize from "cronstrue";

export const TRIGGER_ICONS_OUTLINE: Partial<Record<TriggerType, any>> = {
  [TriggerType.Api]: LinkIconOutline,
  [TriggerType.Edit]: PencilSquareIconOutline,
  [TriggerType.Invoke]: PlayCircleIconOutline,
  [TriggerType.Message]: EnvelopeIconOutline,
  [TriggerType.Run]: PlayCircleIconOutline,
  [TriggerType.Time]: ClockIconOutline,
  [TriggerType.User]: UserCircleIconOutline,
};

export const TRIGGER_ICONS_SOLID: Partial<Record<TriggerType, any>> = {
  [TriggerType.Api]: LinkIconSolid,
  [TriggerType.Edit]: PencilSquareIconSolid,
  [TriggerType.Invoke]: PlayCircleIconSolid,
  [TriggerType.Message]: EnvelopeIconSolid,
  [TriggerType.Run]: PlayCircleIconSolid,
  [TriggerType.Time]: ClockIconSolid,
  [TriggerType.User]: UserCircleIconSolid,
};

export const CONFIGURABLE_TRIGGER_TYPES = [
  TriggerType.Api,
  TriggerType.Time,
  TriggerType.Edit,
  TriggerType.Message,
  TriggerType.Run,
];

// :TriggerSchedule
export const TRIGGER_INTERVAL_ORIGIN = DateTime.fromISO("2022-01-01T00:00:00.000Z");

export const INTERVAL_UNITS = {
  minute: 60,
  hour: 60 * 60,
  day: 60 * 60 * 24,
  week: 60 * 60 * 24 * 7,
};
export type INTERVAL_UNIT = keyof typeof INTERVAL_UNITS;
export const OCCURENCES_PREVIEW = 2;

export type TriggerSchedule = {
  type: ScheduleType;
  timezone: string;
  valid: boolean;
  humanized: string | null;
  now: DateTime;
  lastOccurrence: DateTime | null;
  nextOccurrences: DateTime[];
};

export function getTriggerIntervalUnit(interval: number): INTERVAL_UNIT {
  /* 
  Get the largest interval unit that can be used to represent the given interval (which is in seconds) 
  e.g. 300 -> "minute", 7200 -> "hour", 86400 -> "day", 604800 -> "week"
  */
  for (const unit of ["week", "day", "hour", "minute"] as INTERVAL_UNIT[]) {
    if (interval % INTERVAL_UNITS[unit] == 0) {
      return unit as INTERVAL_UNIT;
    }
  }
  throw new Error(`invalid interval ${interval}`);
}

export function getTriggerSchedule(
  trigger: { timezone: string; scheduleType: ScheduleType; interval: number; cron: string },
  now: DateTime,
  options?: { nextOccurrences?: number }
): TriggerSchedule {
  // :TriggerSchedule
  const { scheduleType, interval, cron } = trigger;
  let valid = false;
  let humanized: string | null = null;
  let lastOccurrence: DateTime | null = null;
  const nextOccurrences: DateTime[] = [];
  const nextOccurrencesCount = options?.nextOccurrences ?? OCCURENCES_PREVIEW;

  if (scheduleType == ScheduleType.Interval) {
    // interval
    valid = true;
    const intervalUnit = getTriggerIntervalUnit(interval);
    humanized = `Every ${interval} ${intervalUnit}s`;
    for (let i = 0; i < nextOccurrencesCount; i++) {
      nextOccurrences.push(now.plus({ seconds: interval }));
    }
    lastOccurrence = now.minus({ seconds: interval });
  } else if (scheduleType == ScheduleType.Cron) {
    // cron
    try {
      const interval = cronInterval.parseExpression(cron);
      valid = true;
      humanized = cronHunaize.toString(cron);

      lastOccurrence = DateTime.fromMillis(interval.prev().getTime());
      for (let i = 0; i < nextOccurrencesCount; i++) {
        const next = interval.next();
        if (next == null) break;
        nextOccurrences.push(DateTime.fromMillis(next.getTime()));
      }
    } catch (e) {
      console.debug("invalid cron", cron, e);
      valid = false;
      // nothing to do
    }
  } else {
    throw new Error(`unexpected schedule type ${scheduleType}`);
  }
  return {
    type: scheduleType,
    valid,
    humanized,
    lastOccurrence,
    nextOccurrences,
  } as TriggerSchedule;
}

export function useTriggerSchedule(
  trigger: Ref<Pick<Trigger, "type" | "scheduleType" | "timezone" | "interval" | "cron"> | null>,
  options?: { nextOccurrences?: number }
) {
  const now = useTimeFromNow(250);
  const schedule: Ref<TriggerSchedule | null> = ref(null);

  watch(
    () => [
      trigger.value?.type,
      trigger.value?.scheduleType,
      trigger.value?.interval,
      trigger.value?.cron,
      now.now.value,
    ],
    () => {
      if (trigger.value?.type != TriggerType.Time) return;
      const { scheduleType, timezone: timezoneMaybe, interval: intervalMaybe, cron: cronMaybe } = trigger.value;
      const timezone = timezoneMaybe ?? "UTC";
      const interval = intervalMaybe ?? 60 * 60;
      const cron = cronMaybe ?? "0 * * * *";
      schedule.value = getTriggerSchedule({ timezone, scheduleType, interval, cron }, now.now.value);
    },
    {
      immediate: true,
    }
  );

  return schedule;
}
