import type { Datetime, Session, UInt8, UInt16, UInt32 } from "@destack/language/core";
import { EnumType, Struct, StructType } from "@destack/language/core";
import { registerEnumClass, registerStructClass } from "@destack/language/registry";
import { hashInt, hashString } from "@destack/utils/hash";

/* ==== DESTACK_GENERATED_START:ENUM:705101 ==== */
/**
 * DayOfWeek
 */
export enum DayOfWeek {
  MONDAY = 1,
  TUESDAY = 2,
  WEDNESDAY = 3,
  THURSDAY = 4,
  FRIDAY = 5,
  SATURDAY = 6,
  SUNDAY = 7,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.DAY_OF_WEEK, DayOfWeek);
/* ==== DESTACK_GENERATED_END:ENUM:705101 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:705102 ==== */
/**
 * Month
 */
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.MONTH, Month);
/* ==== DESTACK_GENERATED_END:ENUM:705102 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:705103 ==== */
/**
 * ScheduleFrequency
 */
export enum ScheduleFrequency {
  YEAR = 1,
  MONTH = 2,
  WEEK = 3,
  DAY = 4,
  HOUR = 5,
  MINUTE = 6,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.SCHEDULE_FREQUENCY, ScheduleFrequency);
/* ==== DESTACK_GENERATED_END:ENUM:705103 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:700001 ==== */
/**
 * The time-based schedule of something (compatible with rrule).
 */
export class Schedule extends Struct {
  static metatype: StructType = StructType.SCHEDULE;
  static __isFrozen__: boolean = false;

  /**
   * Schedule.frequency
   */
  frequency: ScheduleFrequency;

  /**
   * Schedule.interval
   */
  interval: UInt32;

  /**
   * Schedule.start
   */
  start: Datetime | null;

  /**
   * Schedule.end
   */
  end: Datetime | null;

  /**
   * Schedule.count
   */
  count: UInt32 | null;

  /**
   * Schedule.weekStart
   */
  weekStart: DayOfWeek | null;

  /**
   * Schedule.bySetPos
   */
  bySetPos: readonly UInt32[] | null;

  /**
   * Schedule.byMonth
   */
  byMonth: readonly Month[] | null;

  /**
   * Schedule.byMonthDay
   */
  byMonthDay: readonly UInt8[] | null;

  /**
   * Schedule.byYearDay
   */
  byYearDay: readonly UInt16[] | null;

  /**
   * Schedule.byEaster
   */
  byEaster: readonly UInt8[] | null;

  /**
   * Schedule.byWeekNo
   */
  byWeekNo: readonly UInt8[] | null;

  /**
   * Schedule.byWeekDay
   */
  byWeekDay: readonly DayOfWeek[] | null;

  /**
   * Schedule.byHour
   */
  byHour: readonly UInt8[] | null;

  /**
   * Schedule.byMinute
   */
  byMinute: readonly UInt8[] | null;

  /**
   * Schedule.bySecond
   */
  bySecond: readonly UInt8[] | null;

  constructor(options: {
    frequency: ScheduleFrequency;
    interval?: UInt32;
    start?: Datetime | null;
    end?: Datetime | null;
    count?: UInt32 | null;
    weekStart?: DayOfWeek | null;
    bySetPos?: readonly UInt32[] | null;
    byMonth?: readonly Month[] | null;
    byMonthDay?: readonly UInt8[] | null;
    byYearDay?: readonly UInt16[] | null;
    byEaster?: readonly UInt8[] | null;
    byWeekNo?: readonly UInt8[] | null;
    byWeekDay?: readonly DayOfWeek[] | null;
    byHour?: readonly UInt8[] | null;
    byMinute?: readonly UInt8[] | null;
    bySecond?: readonly UInt8[] | null;
    _session?: Session | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _frequency = options.frequency;
    if (_frequency == null) {
      throw new Error(`Schedule.frequency is required`);
    }
    this.frequency = _frequency;
    let _interval = options.interval ?? null;
    if (_interval == null) {
      _interval = 1;
    }
    if (_interval == null) {
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
    this.bySetPos = _bySetPos;
    let _byMonth = options.byMonth ?? null;
    this.byMonth = _byMonth;
    let _byMonthDay = options.byMonthDay ?? null;
    this.byMonthDay = _byMonthDay;
    let _byYearDay = options.byYearDay ?? null;
    this.byYearDay = _byYearDay;
    let _byEaster = options.byEaster ?? null;
    this.byEaster = _byEaster;
    let _byWeekNo = options.byWeekNo ?? null;
    this.byWeekNo = _byWeekNo;
    let _byWeekDay = options.byWeekDay ?? null;
    this.byWeekDay = _byWeekDay;
    let _byHour = options.byHour ?? null;
    this.byHour = _byHour;
    let _byMinute = options.byMinute ?? null;
    this.byMinute = _byMinute;
    let _bySecond = options.bySecond ?? null;
    this.bySecond = _bySecond;

    /* identity */
    /* ... */
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.frequency === other.frequency)) {
      return false;
    }
    if (!(this.interval === other.interval)) {
      return false;
    }
    if (!(this.start === other.start)) {
      return false;
    }
    if (!(this.end === other.end)) {
      return false;
    }
    if (!(this.count === other.count)) {
      return false;
    }
    if (!(this.weekStart === other.weekStart)) {
      return false;
    }
    if (this.bySetPos == null) {
      return other.bySetPos == null;
    }
    if (this.bySetPos.length != other.bySetPos.length) {
      return false;
    }
    for (let i = 0; i < this.bySetPos.length; i++) {
      if (!(this.bySetPos[i] === other.bySetPos[i])) {
        return false;
      }
    }

    if (this.byMonth == null) {
      return other.byMonth == null;
    }
    if (this.byMonth.length != other.byMonth.length) {
      return false;
    }
    for (let i = 0; i < this.byMonth.length; i++) {
      if (!(this.byMonth[i] === other.byMonth[i])) {
        return false;
      }
    }

    if (this.byMonthDay == null) {
      return other.byMonthDay == null;
    }
    if (this.byMonthDay.length != other.byMonthDay.length) {
      return false;
    }
    for (let i = 0; i < this.byMonthDay.length; i++) {
      if (!(this.byMonthDay[i] === other.byMonthDay[i])) {
        return false;
      }
    }

    if (this.byYearDay == null) {
      return other.byYearDay == null;
    }
    if (this.byYearDay.length != other.byYearDay.length) {
      return false;
    }
    for (let i = 0; i < this.byYearDay.length; i++) {
      if (!(this.byYearDay[i] === other.byYearDay[i])) {
        return false;
      }
    }

    if (this.byEaster == null) {
      return other.byEaster == null;
    }
    if (this.byEaster.length != other.byEaster.length) {
      return false;
    }
    for (let i = 0; i < this.byEaster.length; i++) {
      if (!(this.byEaster[i] === other.byEaster[i])) {
        return false;
      }
    }

    if (this.byWeekNo == null) {
      return other.byWeekNo == null;
    }
    if (this.byWeekNo.length != other.byWeekNo.length) {
      return false;
    }
    for (let i = 0; i < this.byWeekNo.length; i++) {
      if (!(this.byWeekNo[i] === other.byWeekNo[i])) {
        return false;
      }
    }

    if (this.byWeekDay == null) {
      return other.byWeekDay == null;
    }
    if (this.byWeekDay.length != other.byWeekDay.length) {
      return false;
    }
    for (let i = 0; i < this.byWeekDay.length; i++) {
      if (!(this.byWeekDay[i] === other.byWeekDay[i])) {
        return false;
      }
    }

    if (this.byHour == null) {
      return other.byHour == null;
    }
    if (this.byHour.length != other.byHour.length) {
      return false;
    }
    for (let i = 0; i < this.byHour.length; i++) {
      if (!(this.byHour[i] === other.byHour[i])) {
        return false;
      }
    }

    if (this.byMinute == null) {
      return other.byMinute == null;
    }
    if (this.byMinute.length != other.byMinute.length) {
      return false;
    }
    for (let i = 0; i < this.byMinute.length; i++) {
      if (!(this.byMinute[i] === other.byMinute[i])) {
        return false;
      }
    }

    if (this.bySecond == null) {
      return other.bySecond == null;
    }
    if (this.bySecond.length != other.bySecond.length) {
      return false;
    }
    for (let i = 0; i < this.bySecond.length; i++) {
      if (!(this.bySecond[i] === other.bySecond[i])) {
        return false;
      }
    }

    return true;
  }

  repr(): string {
    return `<Schedule>`;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.frequency) & 0xffffffff;
    h = (h * 31 + hashInt(this.interval)) & 0xffffffff;
    if (this.start != null) {
      h = (h * 31 + hashString(this.start.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.end != null) {
      h = (h * 31 + hashString(this.end.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.count != null) {
      h = (h * 31 + hashInt(this.count)) & 0xffffffff;
    }
    if (this.weekStart != null) {
      h = (h * 31 + this.weekStart) & 0xffffffff;
    }
    if (this.bySetPos && this.bySetPos.length > 0) {
      for (const _item of this.bySetPos) {
        h = (h * 31 + hashInt(_item)) & 0xffffffff;
      }
    }
    if (this.byMonth && this.byMonth.length > 0) {
      for (const _item of this.byMonth) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.byMonthDay && this.byMonthDay.length > 0) {
      for (const _item of this.byMonthDay) {
        h = (h * 31 + hashInt(_item)) & 0xffffffff;
      }
    }
    if (this.byYearDay && this.byYearDay.length > 0) {
      for (const _item of this.byYearDay) {
        h = (h * 31 + hashInt(_item)) & 0xffffffff;
      }
    }
    if (this.byEaster && this.byEaster.length > 0) {
      for (const _item of this.byEaster) {
        h = (h * 31 + hashInt(_item)) & 0xffffffff;
      }
    }
    if (this.byWeekNo && this.byWeekNo.length > 0) {
      for (const _item of this.byWeekNo) {
        h = (h * 31 + hashInt(_item)) & 0xffffffff;
      }
    }
    if (this.byWeekDay && this.byWeekDay.length > 0) {
      for (const _item of this.byWeekDay) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.byHour && this.byHour.length > 0) {
      for (const _item of this.byHour) {
        h = (h * 31 + hashInt(_item)) & 0xffffffff;
      }
    }
    if (this.byMinute && this.byMinute.length > 0) {
      for (const _item of this.byMinute) {
        h = (h * 31 + hashInt(_item)) & 0xffffffff;
      }
    }
    if (this.bySecond && this.bySecond.length > 0) {
      for (const _item of this.bySecond) {
        h = (h * 31 + hashInt(_item)) & 0xffffffff;
      }
    }

    return h;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.SCHEDULE, Schedule);
/* ==== DESTACK_GENERATED_END:STRUCT:700001 ==== */
