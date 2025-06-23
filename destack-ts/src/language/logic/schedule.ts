import { Session, Struct, StructType, Supergraph } from "@destack/language";
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
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _frequency = options.frequency;
    if (_frequency === null) {
      throw new Error(`Schedule.frequency is required`);
    }
    this.frequency = _frequency;
    let _interval = options.interval ?? null;
    if (_interval === null) {
      _interval = 1;
    }
    if (_interval === null) {
      throw new Error(`Schedule.interval is required`);
    }
    this.interval = _interval;
    let _start = options.start ?? null;
    this.start = _start;
    let _end = options.end ?? null;
    this.end = _end;
    let _count = options.count ?? null;
    this.count = _count;
    let _weekStart = options.weekStart ?? null;
    this.weekStart = _weekStart;
    let _bySetPos = options.bySetPos ?? null;
    if (_bySetPos === null) {
      throw new Error(`Schedule.bySetPos is required`);
    }
    this.bySetPos = _bySetPos;
    let _byMonth = options.byMonth ?? null;
    if (_byMonth === null) {
      throw new Error(`Schedule.byMonth is required`);
    }
    this.byMonth = _byMonth;
    let _byMonthDay = options.byMonthDay ?? null;
    if (_byMonthDay === null) {
      throw new Error(`Schedule.byMonthDay is required`);
    }
    this.byMonthDay = _byMonthDay;
    let _byYearDay = options.byYearDay ?? null;
    if (_byYearDay === null) {
      throw new Error(`Schedule.byYearDay is required`);
    }
    this.byYearDay = _byYearDay;
    let _byEaster = options.byEaster ?? null;
    if (_byEaster === null) {
      throw new Error(`Schedule.byEaster is required`);
    }
    this.byEaster = _byEaster;
    let _byWeekNo = options.byWeekNo ?? null;
    if (_byWeekNo === null) {
      throw new Error(`Schedule.byWeekNo is required`);
    }
    this.byWeekNo = _byWeekNo;
    let _byWeekDay = options.byWeekDay ?? null;
    if (_byWeekDay === null) {
      throw new Error(`Schedule.byWeekDay is required`);
    }
    this.byWeekDay = _byWeekDay;
    let _byHour = options.byHour ?? null;
    if (_byHour === null) {
      throw new Error(`Schedule.byHour is required`);
    }
    this.byHour = _byHour;
    let _byMinute = options.byMinute ?? null;
    if (_byMinute === null) {
      throw new Error(`Schedule.byMinute is required`);
    }
    this.byMinute = _byMinute;
    let _bySecond = options.bySecond ?? null;
    if (_bySecond === null) {
      throw new Error(`Schedule.bySecond is required`);
    }
    this.bySecond = _bySecond;

    // identity
    // ...
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
