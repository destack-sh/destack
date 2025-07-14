import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type { Session, Supergraph } from "@destack/language/core";
import { EnumType, Struct, StructType } from "@destack/language/core";
import { registerEnumClass, registerStructClass } from "@destack/language/registry";
import { DayOfWeekProto, MonthProto, ScheduleFrequencyProto, ScheduleProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashInt, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:STRUCT:100001 ==== */
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
  bySetPos: readonly number[];

  /**
   * Schedule.byMonth
   */
  byMonth: readonly Month[];

  /**
   * Schedule.byMonthDay
   */
  byMonthDay: readonly number[];

  /**
   * Schedule.byYearDay
   */
  byYearDay: readonly number[];

  /**
   * Schedule.byEaster
   */
  byEaster: readonly number[];

  /**
   * Schedule.byWeekNo
   */
  byWeekNo: readonly number[];

  /**
   * Schedule.byWeekDay
   */
  byWeekDay: readonly DayOfWeek[];

  /**
   * Schedule.byHour
   */
  byHour: readonly number[];

  /**
   * Schedule.byMinute
   */
  byMinute: readonly number[];

  /**
   * Schedule.bySecond
   */
  bySecond: readonly number[];

  constructor(options: {
    frequency: ScheduleFrequency;
    interval?: number;
    start?: Temporal.ZonedDateTime | null;
    end?: Temporal.ZonedDateTime | null;
    count?: number | null;
    weekStart?: DayOfWeek | null;
    bySetPos?: readonly number[];
    byMonth?: readonly Month[];
    byMonthDay?: readonly number[];
    byYearDay?: readonly number[];
    byEaster?: readonly number[];
    byWeekNo?: readonly number[];
    byWeekDay?: readonly DayOfWeek[];
    byHour?: readonly number[];
    byMinute?: readonly number[];
    bySecond?: readonly number[];
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
      _bySetPos = [];
    }
    this.bySetPos = _bySetPos;
    let _byMonth = options.byMonth ?? null;
    if (_byMonth === null) {
      _byMonth = [];
    }
    this.byMonth = _byMonth;
    let _byMonthDay = options.byMonthDay ?? null;
    if (_byMonthDay === null) {
      _byMonthDay = [];
    }
    this.byMonthDay = _byMonthDay;
    let _byYearDay = options.byYearDay ?? null;
    if (_byYearDay === null) {
      _byYearDay = [];
    }
    this.byYearDay = _byYearDay;
    let _byEaster = options.byEaster ?? null;
    if (_byEaster === null) {
      _byEaster = [];
    }
    this.byEaster = _byEaster;
    let _byWeekNo = options.byWeekNo ?? null;
    if (_byWeekNo === null) {
      _byWeekNo = [];
    }
    this.byWeekNo = _byWeekNo;
    let _byWeekDay = options.byWeekDay ?? null;
    if (_byWeekDay === null) {
      _byWeekDay = [];
    }
    this.byWeekDay = _byWeekDay;
    let _byHour = options.byHour ?? null;
    if (_byHour === null) {
      _byHour = [];
    }
    this.byHour = _byHour;
    let _byMinute = options.byMinute ?? null;
    if (_byMinute === null) {
      _byMinute = [];
    }
    this.byMinute = _byMinute;
    let _bySecond = options.bySecond ?? null;
    if (_bySecond === null) {
      _bySecond = [];
    }
    this.bySecond = _bySecond;

    // identity
    // ...
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
    if (this.bySetPos.length != other.bySetPos.length) {
      return false;
    }
    for (let i = 0; i < this.bySetPos.length; i++) {
      if (!(this.bySetPos[i] === other.bySetPos[i])) {
        return false;
      }
    }
    if (this.byMonth.length != other.byMonth.length) {
      return false;
    }
    for (let i = 0; i < this.byMonth.length; i++) {
      if (!(this.byMonth[i] === other.byMonth[i])) {
        return false;
      }
    }
    if (this.byMonthDay.length != other.byMonthDay.length) {
      return false;
    }
    for (let i = 0; i < this.byMonthDay.length; i++) {
      if (!(this.byMonthDay[i] === other.byMonthDay[i])) {
        return false;
      }
    }
    if (this.byYearDay.length != other.byYearDay.length) {
      return false;
    }
    for (let i = 0; i < this.byYearDay.length; i++) {
      if (!(this.byYearDay[i] === other.byYearDay[i])) {
        return false;
      }
    }
    if (this.byEaster.length != other.byEaster.length) {
      return false;
    }
    for (let i = 0; i < this.byEaster.length; i++) {
      if (!(this.byEaster[i] === other.byEaster[i])) {
        return false;
      }
    }
    if (this.byWeekNo.length != other.byWeekNo.length) {
      return false;
    }
    for (let i = 0; i < this.byWeekNo.length; i++) {
      if (!(this.byWeekNo[i] === other.byWeekNo[i])) {
        return false;
      }
    }
    if (this.byWeekDay.length != other.byWeekDay.length) {
      return false;
    }
    for (let i = 0; i < this.byWeekDay.length; i++) {
      if (!(this.byWeekDay[i] === other.byWeekDay[i])) {
        return false;
      }
    }
    if (this.byHour.length != other.byHour.length) {
      return false;
    }
    for (let i = 0; i < this.byHour.length; i++) {
      if (!(this.byHour[i] === other.byHour[i])) {
        return false;
      }
    }
    if (this.byMinute.length != other.byMinute.length) {
      return false;
    }
    for (let i = 0; i < this.byMinute.length; i++) {
      if (!(this.byMinute[i] === other.byMinute[i])) {
        return false;
      }
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

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { readonly [key: string]: any } {
    return Schedule.__packValue__(this);
  }

  static __packValue__(object: Schedule): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 100001;
    objectValue["101"] = object.frequency;
    objectValue["102"] = object.interval;
    if (object.start != null) {
      objectValue["110"] = object.start.toString({ timeZoneName: "never" });
    }
    if (object.end != null) {
      objectValue["111"] = object.end.toString({ timeZoneName: "never" });
    }
    if (object.count != null) {
      objectValue["112"] = object.count;
    }
    if (object.weekStart != null) {
      objectValue["113"] = object.weekStart;
    }
    if (object.bySetPos.length > 0) {
      const packedBySetPos: any[] = [];
      for (const item of object.bySetPos) {
        packedBySetPos.push(item);
      }
      objectValue["114"] = packedBySetPos;
    }
    if (object.byMonth.length > 0) {
      const packedByMonth: any[] = [];
      for (const item of object.byMonth) {
        packedByMonth.push(item);
      }
      objectValue["115"] = packedByMonth;
    }
    if (object.byMonthDay.length > 0) {
      const packedByMonthDay: any[] = [];
      for (const item of object.byMonthDay) {
        packedByMonthDay.push(item);
      }
      objectValue["116"] = packedByMonthDay;
    }
    if (object.byYearDay.length > 0) {
      const packedByYearDay: any[] = [];
      for (const item of object.byYearDay) {
        packedByYearDay.push(item);
      }
      objectValue["117"] = packedByYearDay;
    }
    if (object.byEaster.length > 0) {
      const packedByEaster: any[] = [];
      for (const item of object.byEaster) {
        packedByEaster.push(item);
      }
      objectValue["118"] = packedByEaster;
    }
    if (object.byWeekNo.length > 0) {
      const packedByWeekNo: any[] = [];
      for (const item of object.byWeekNo) {
        packedByWeekNo.push(item);
      }
      objectValue["119"] = packedByWeekNo;
    }
    if (object.byWeekDay.length > 0) {
      const packedByWeekDay: any[] = [];
      for (const item of object.byWeekDay) {
        packedByWeekDay.push(item);
      }
      objectValue["120"] = packedByWeekDay;
    }
    if (object.byHour.length > 0) {
      const packedByHour: any[] = [];
      for (const item of object.byHour) {
        packedByHour.push(item);
      }
      objectValue["121"] = packedByHour;
    }
    if (object.byMinute.length > 0) {
      const packedByMinute: any[] = [];
      for (const item of object.byMinute) {
        packedByMinute.push(item);
      }
      objectValue["122"] = packedByMinute;
    }
    if (object.bySecond.length > 0) {
      const packedBySecond: any[] = [];
      for (const item of object.bySecond) {
        packedBySecond.push(item);
      }
      objectValue["123"] = packedBySecond;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Schedule {
    const startValue = objectValue["110"];
    const unpackedStart =
      startValue != undefined ? Temporal.Instant.from(startValue).toZonedDateTimeISO("UTC") : null;
    const endValue = objectValue["111"];
    const unpackedEnd =
      endValue != undefined ? Temporal.Instant.from(endValue).toZonedDateTimeISO("UTC") : null;
    const countValue = objectValue["112"];
    const unpackedCount = countValue != undefined ? Number(countValue) : null;
    const weekStartValue = objectValue["113"];
    const unpackedWeekStart = weekStartValue != undefined ? Number(weekStartValue) : null;
    const unpackedBySetPos: any[] = [];
    if (objectValue["114"] != undefined) {
      for (const item of objectValue["114"]) {
        unpackedBySetPos.push(Number(item));
      }
    }
    const unpackedByMonth: any[] = [];
    if (objectValue["115"] != undefined) {
      for (const item of objectValue["115"]) {
        unpackedByMonth.push(Number(item));
      }
    }
    const unpackedByMonthDay: any[] = [];
    if (objectValue["116"] != undefined) {
      for (const item of objectValue["116"]) {
        unpackedByMonthDay.push(Number(item));
      }
    }
    const unpackedByYearDay: any[] = [];
    if (objectValue["117"] != undefined) {
      for (const item of objectValue["117"]) {
        unpackedByYearDay.push(Number(item));
      }
    }
    const unpackedByEaster: any[] = [];
    if (objectValue["118"] != undefined) {
      for (const item of objectValue["118"]) {
        unpackedByEaster.push(Number(item));
      }
    }
    const unpackedByWeekNo: any[] = [];
    if (objectValue["119"] != undefined) {
      for (const item of objectValue["119"]) {
        unpackedByWeekNo.push(Number(item));
      }
    }
    const unpackedByWeekDay: any[] = [];
    if (objectValue["120"] != undefined) {
      for (const item of objectValue["120"]) {
        unpackedByWeekDay.push(Number(item));
      }
    }
    const unpackedByHour: any[] = [];
    if (objectValue["121"] != undefined) {
      for (const item of objectValue["121"]) {
        unpackedByHour.push(Number(item));
      }
    }
    const unpackedByMinute: any[] = [];
    if (objectValue["122"] != undefined) {
      for (const item of objectValue["122"]) {
        unpackedByMinute.push(Number(item));
      }
    }
    const unpackedBySecond: any[] = [];
    if (objectValue["123"] != undefined) {
      for (const item of objectValue["123"]) {
        unpackedBySecond.push(Number(item));
      }
    }
    return new Schedule({
      frequency: Number(objectValue["101"]),
      interval: Number(objectValue["102"]),
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
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Schedule {
    return Schedule.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): ScheduleProto {
    return Schedule.__packProto__(this);
  }

  static __packProto__(object: Schedule): ScheduleProto {
    const objectProto: Partial<ScheduleProto> = { metatype: 100001 };
    objectProto.frequency = Number(object.frequency) as ScheduleFrequencyProto;
    objectProto.interval = object.interval;
    if (object.start != null) {
      objectProto.start = packProtoTimestamp(object.start);
    }
    if (object.end != null) {
      objectProto.end = packProtoTimestamp(object.end);
    }
    if (object.count != null) {
      objectProto.count = object.count;
    }
    if (object.weekStart != null) {
      objectProto.weekStart = Number(object.weekStart) as DayOfWeekProto;
    }
    if (object.bySetPos) {
      const packedBySetPos: any[] = [];
      for (const item of object.bySetPos) {
        packedBySetPos.push(item);
      }
      objectProto.bySetPos = packedBySetPos;
    }
    if (object.byMonth) {
      const packedByMonth: any[] = [];
      for (const item of object.byMonth) {
        packedByMonth.push(Number(item) as MonthProto);
      }
      objectProto.byMonth = packedByMonth;
    }
    if (object.byMonthDay) {
      const packedByMonthDay: any[] = [];
      for (const item of object.byMonthDay) {
        packedByMonthDay.push(item);
      }
      objectProto.byMonthDay = packedByMonthDay;
    }
    if (object.byYearDay) {
      const packedByYearDay: any[] = [];
      for (const item of object.byYearDay) {
        packedByYearDay.push(item);
      }
      objectProto.byYearDay = packedByYearDay;
    }
    if (object.byEaster) {
      const packedByEaster: any[] = [];
      for (const item of object.byEaster) {
        packedByEaster.push(item);
      }
      objectProto.byEaster = packedByEaster;
    }
    if (object.byWeekNo) {
      const packedByWeekNo: any[] = [];
      for (const item of object.byWeekNo) {
        packedByWeekNo.push(item);
      }
      objectProto.byWeekNo = packedByWeekNo;
    }
    if (object.byWeekDay) {
      const packedByWeekDay: any[] = [];
      for (const item of object.byWeekDay) {
        packedByWeekDay.push(Number(item) as DayOfWeekProto);
      }
      objectProto.byWeekDay = packedByWeekDay;
    }
    if (object.byHour) {
      const packedByHour: any[] = [];
      for (const item of object.byHour) {
        packedByHour.push(item);
      }
      objectProto.byHour = packedByHour;
    }
    if (object.byMinute) {
      const packedByMinute: any[] = [];
      for (const item of object.byMinute) {
        packedByMinute.push(item);
      }
      objectProto.byMinute = packedByMinute;
    }
    if (object.bySecond) {
      const packedBySecond: any[] = [];
      for (const item of object.bySecond) {
        packedBySecond.push(item);
      }
      objectProto.bySecond = packedBySecond;
    }
    return objectProto as ScheduleProto;
  }

  static __unpackProto__(
    objectProto: ScheduleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Schedule {
    const unpackedBySetPos: any[] = [];
    if (objectProto.bySetPos) {
      for (const item of objectProto.bySetPos) {
        unpackedBySetPos.push(Number(item));
      }
    }
    const unpackedByMonth: any[] = [];
    if (objectProto.byMonth) {
      for (const item of objectProto.byMonth) {
        unpackedByMonth.push(Number(item) as Month);
      }
    }
    const unpackedByMonthDay: any[] = [];
    if (objectProto.byMonthDay) {
      for (const item of objectProto.byMonthDay) {
        unpackedByMonthDay.push(Number(item));
      }
    }
    const unpackedByYearDay: any[] = [];
    if (objectProto.byYearDay) {
      for (const item of objectProto.byYearDay) {
        unpackedByYearDay.push(Number(item));
      }
    }
    const unpackedByEaster: any[] = [];
    if (objectProto.byEaster) {
      for (const item of objectProto.byEaster) {
        unpackedByEaster.push(Number(item));
      }
    }
    const unpackedByWeekNo: any[] = [];
    if (objectProto.byWeekNo) {
      for (const item of objectProto.byWeekNo) {
        unpackedByWeekNo.push(Number(item));
      }
    }
    const unpackedByWeekDay: any[] = [];
    if (objectProto.byWeekDay) {
      for (const item of objectProto.byWeekDay) {
        unpackedByWeekDay.push(Number(item) as DayOfWeek);
      }
    }
    const unpackedByHour: any[] = [];
    if (objectProto.byHour) {
      for (const item of objectProto.byHour) {
        unpackedByHour.push(Number(item));
      }
    }
    const unpackedByMinute: any[] = [];
    if (objectProto.byMinute) {
      for (const item of objectProto.byMinute) {
        unpackedByMinute.push(Number(item));
      }
    }
    const unpackedBySecond: any[] = [];
    if (objectProto.bySecond) {
      for (const item of objectProto.bySecond) {
        unpackedBySecond.push(Number(item));
      }
    }
    return new Schedule({
      frequency: Number(objectProto.frequency) as ScheduleFrequency,
      interval: Number(objectProto.interval),
      start: objectProto.start != undefined ? unpackProtoTimestamp(objectProto.start!) : null,
      end: objectProto.end != undefined ? unpackProtoTimestamp(objectProto.end!) : null,
      count: objectProto.count != undefined ? Number(objectProto.count) : null,
      weekStart:
        objectProto.weekStart != undefined ? (Number(objectProto.weekStart) as DayOfWeek) : null,
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

  static fromProto(
    objectProto: ScheduleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Schedule {
    return Schedule.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Schedule {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ScheduleProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.SCHEDULE, Schedule);
/* ==== DESTACK_GENERATED_END:STRUCT:100001 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:105101 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:105101 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:105102 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:105102 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:105103 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:105103 ==== */
