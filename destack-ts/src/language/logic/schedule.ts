import { Session, Struct, StructType, Supergraph } from "@destack/language/core";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:3051 ==== */
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
}
/* ==== DESTACK_GENERATED_END:ENUM:3051 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:3052 ==== */
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
}
/* ==== DESTACK_GENERATED_END:ENUM:3052 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:3050 ==== */
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
}
/* ==== DESTACK_GENERATED_END:ENUM:3050 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:3001 ==== */
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
  interval: number;

  /**
   * Schedule.start
   */
  start: Temporal.ZonedDateTime | null;

  /**
   * Schedule.end
   */
  end: Temporal.ZonedDateTime | null;

  /**
   * Schedule.count
   */
  count: number | null;

  /**
   * Schedule.weekStart
   */
  weekStart: DayOfWeek | null;

  /**
   * Schedule.bySetPos
   */
  bySetPos: Array<number>;

  /**
   * Schedule.byMonth
   */
  byMonth: Array<Month>;

  /**
   * Schedule.byMonthDay
   */
  byMonthDay: Array<number>;

  /**
   * Schedule.byYearDay
   */
  byYearDay: Array<number>;

  /**
   * Schedule.byEaster
   */
  byEaster: Array<number>;

  /**
   * Schedule.byWeekNo
   */
  byWeekNo: Array<number>;

  /**
   * Schedule.byWeekDay
   */
  byWeekDay: Array<DayOfWeek>;

  /**
   * Schedule.byHour
   */
  byHour: Array<number>;

  /**
   * Schedule.byMinute
   */
  byMinute: Array<number>;

  /**
   * Schedule.bySecond
   */
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

  toValue(): { [key: string]: any } {
    return Schedule.__packValue__(this);
  }

  static __packValue__(object: Schedule): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 3001;
    objectValue["31"] = object.frequency;
    objectValue["32"] = object.interval;
    if (object.start !== null) {
      objectValue["33"] = object.start.toString();
    }
    if (object.end !== null) {
      objectValue["34"] = object.end.toString();
    }
    if (object.count !== null) {
      objectValue["35"] = object.count;
    }
    if (object.weekStart !== null) {
      objectValue["36"] = object.weekStart;
    }
    if (object.bySetPos) {
      const packedBySetPos: any[] = [];
      for (const item of object.bySetPos) {
        packedBySetPos.push(item);
      }
      objectValue["37"] = packedBySetPos;
    }
    if (object.byMonth) {
      const packedByMonth: any[] = [];
      for (const item of object.byMonth) {
        packedByMonth.push(item);
      }
      objectValue["38"] = packedByMonth;
    }
    if (object.byMonthDay) {
      const packedByMonthDay: any[] = [];
      for (const item of object.byMonthDay) {
        packedByMonthDay.push(item);
      }
      objectValue["39"] = packedByMonthDay;
    }
    if (object.byYearDay) {
      const packedByYearDay: any[] = [];
      for (const item of object.byYearDay) {
        packedByYearDay.push(item);
      }
      objectValue["40"] = packedByYearDay;
    }
    if (object.byEaster) {
      const packedByEaster: any[] = [];
      for (const item of object.byEaster) {
        packedByEaster.push(item);
      }
      objectValue["41"] = packedByEaster;
    }
    if (object.byWeekNo) {
      const packedByWeekNo: any[] = [];
      for (const item of object.byWeekNo) {
        packedByWeekNo.push(item);
      }
      objectValue["42"] = packedByWeekNo;
    }
    if (object.byWeekDay) {
      const packedByWeekDay: any[] = [];
      for (const item of object.byWeekDay) {
        packedByWeekDay.push(item);
      }
      objectValue["43"] = packedByWeekDay;
    }
    if (object.byHour) {
      const packedByHour: any[] = [];
      for (const item of object.byHour) {
        packedByHour.push(item);
      }
      objectValue["44"] = packedByHour;
    }
    if (object.byMinute) {
      const packedByMinute: any[] = [];
      for (const item of object.byMinute) {
        packedByMinute.push(item);
      }
      objectValue["45"] = packedByMinute;
    }
    if (object.bySecond) {
      const packedBySecond: any[] = [];
      for (const item of object.bySecond) {
        packedBySecond.push(item);
      }
      objectValue["46"] = packedBySecond;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Schedule {
    const startValue = objectValue["33"];
    const unpackedStart = startValue !== undefined ? Temporal.ZonedDateTime.from(startValue) : null;
    const endValue = objectValue["34"];
    const unpackedEnd = endValue !== undefined ? Temporal.ZonedDateTime.from(endValue) : null;
    const countValue = objectValue["35"];
    const unpackedCount = countValue !== undefined ? Number(countValue) : null;
    const weekStartValue = objectValue["36"];
    const unpackedWeekStart = weekStartValue !== undefined ? Number(weekStartValue) : null;
    const unpackedBySetPos: any[] = [];
    if (objectValue["37"] !== undefined) {
      for (const item of objectValue["37"]) {
        unpackedBySetPos.push(Number(item));
      }
    }
    const unpackedByMonth: any[] = [];
    if (objectValue["38"] !== undefined) {
      for (const item of objectValue["38"]) {
        unpackedByMonth.push(Number(item));
      }
    }
    const unpackedByMonthDay: any[] = [];
    if (objectValue["39"] !== undefined) {
      for (const item of objectValue["39"]) {
        unpackedByMonthDay.push(Number(item));
      }
    }
    const unpackedByYearDay: any[] = [];
    if (objectValue["40"] !== undefined) {
      for (const item of objectValue["40"]) {
        unpackedByYearDay.push(Number(item));
      }
    }
    const unpackedByEaster: any[] = [];
    if (objectValue["41"] !== undefined) {
      for (const item of objectValue["41"]) {
        unpackedByEaster.push(Number(item));
      }
    }
    const unpackedByWeekNo: any[] = [];
    if (objectValue["42"] !== undefined) {
      for (const item of objectValue["42"]) {
        unpackedByWeekNo.push(Number(item));
      }
    }
    const unpackedByWeekDay: any[] = [];
    if (objectValue["43"] !== undefined) {
      for (const item of objectValue["43"]) {
        unpackedByWeekDay.push(Number(item));
      }
    }
    const unpackedByHour: any[] = [];
    if (objectValue["44"] !== undefined) {
      for (const item of objectValue["44"]) {
        unpackedByHour.push(Number(item));
      }
    }
    const unpackedByMinute: any[] = [];
    if (objectValue["45"] !== undefined) {
      for (const item of objectValue["45"]) {
        unpackedByMinute.push(Number(item));
      }
    }
    const unpackedBySecond: any[] = [];
    if (objectValue["46"] !== undefined) {
      for (const item of objectValue["46"]) {
        unpackedBySecond.push(Number(item));
      }
    }
    return new Schedule({
      frequency: Number(objectValue["31"]),
      interval: Number(objectValue["32"]),
      start: unpackedStart,
      end: unpackedEnd,
      count: unpackedCount,
      weekStart: unpackedWeekStart,
      bySetPos: unpackedBySetPos,
      byMonth: unpackedByMonth,
      byMonthDay: unpackedByMonthDay,
      byYearDay: unpackedByYearDay,
      byEaster: unpackedByEaster,
      byWeekNo: unpackedByWeekNo,
      byWeekDay: unpackedByWeekDay,
      byHour: unpackedByHour,
      byMinute: unpackedByMinute,
      bySecond: unpackedBySecond,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Schedule {
    return Schedule.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:3001 ==== */
