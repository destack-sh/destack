import { Session, Struct, StructType, Supergraph } from "@/language";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:3051 ==== */
export enum DayOfWeek {
  MONDAY = 1,
  TUESDAY = 2,
  WEDNESDAY = 3,
  THURSDAY = 4,
  FRIDAY = 5,
  SATURDAY = 6,
  SUNDAY = 7,
}
/* ==== DESTACK_GENERATED_END:ENUM:3051 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:3052 ==== */
export enum Month {
  JANUARY = 1,
  FEBRUARY = 2,
  MARCH = 3,
  APRIL = 4,
  MAY = 5,
  JUNE = 6,
  JULY = 7,
  AUGUST = 8,
  SEPTEMBER = 9,
  OCTOBER = 10,
  NOVEMBER = 11,
  DECEMBER = 12,
}
/* ==== DESTACK_GENERATED_END:ENUM:3052 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:3050 ==== */
export enum ScheduleFrequency {
  YEAR = 1,
  MONTH = 2,
  WEEK = 3,
  DAY = 4,
  HOUR = 5,
  MINUTE = 6,
}
/* ==== DESTACK_GENERATED_END:ENUM:3050 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:3001 ==== */
export class Schedule extends Struct {
  static metatype: StructType = StructType.SCHEDULE;
  static __isFrozen__: boolean = false;

  frequency: ScheduleFrequency;
  interval: number;
  start: Temporal.ZonedDateTime | null;
  end: Temporal.ZonedDateTime | null;
  count: number | null;
  weekStart: DayOfWeek | null;
  bySetPos: Array<number>;
  byMonth: Array<Month>;
  byMonthDay: Array<number>;
  byYearDay: Array<number>;
  byEaster: Array<number>;
  byWeekNo: Array<number>;
  byWeekDay: Array<DayOfWeek>;
  byHour: Array<number>;
  byMinute: Array<number>;
  bySecond: Array<number>;

  constructor(options: {
    frequency: ScheduleFrequency;
    interval?: number;
    start?: Temporal.ZonedDateTime | null;
    end?: Temporal.ZonedDateTime | null;
    count?: number | null;
    weekStart?: DayOfWeek | null;
    bySetPos?: Array<number>;
    byMonth?: Array<Month>;
    byMonthDay?: Array<number>;
    byYearDay?: Array<number>;
    byEaster?: Array<number>;
    byWeekNo?: Array<number>;
    byWeekDay?: Array<DayOfWeek>;
    byHour?: Array<number>;
    byMinute?: Array<number>;
    bySecond?: Array<number>;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // supergraph
      options._supergraph ?? null,
    );

    this.frequency = options.frequency;
    this.interval = options.interval ?? 1;
    this.start = options.start ?? null;
    this.end = options.end ?? null;
    this.count = options.count ?? null;
    this.weekStart = options.weekStart ?? null;
    this.bySetPos = options.bySetPos ?? [];
    this.byMonth = options.byMonth ?? [];
    this.byMonthDay = options.byMonthDay ?? [];
    this.byYearDay = options.byYearDay ?? [];
    this.byEaster = options.byEaster ?? [];
    this.byWeekNo = options.byWeekNo ?? [];
    this.byWeekDay = options.byWeekDay ?? [];
    this.byHour = options.byHour ?? [];
    this.byMinute = options.byMinute ?? [];
    this.bySecond = options.bySecond ?? [];
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:3001 ==== */
