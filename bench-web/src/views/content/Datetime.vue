<script lang="ts" setup>
import { ViewData, NodeType, ViewType, PrimitiveType, type Timestamp, RectangleData, ObjectType } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { ViewContentWrapper, viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { computed, Ref, ref, toRef, watch } from "vue";
import { DateTime, WeekdayNumbers } from "luxon";
import { ICON_BY_PRIMITIVE_TYPE, IconInline, makeIcon } from "@/ui/icon";
import type { PopoverInfoIn } from "@/ui/popover";
import { tsToDt, dtToTs, formatAbsoluteDate } from "@/utils/time";
import type { Date as ProtoDate } from "@/proto/wire/proto/google/type/date";
import type { TimeOfDay } from "@/proto/wire/proto/google/type/timeofday";

const MIN_WIDTH = 320;
const DEFAULT_WIDTH = 400;
const MAX_HEIGHT = 360;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    size?: Partial<Pick<RectangleData, "width" | "height">>;
    modelValue?: Timestamp | ProtoDate | TimeOfDay;
    isPopover?: boolean;
  } & Partial<
    Pick<
      ViewData,
      "name" | "title" | "icon" | "valueType" | "nodePtr" | "isInput" | "isDisabled" | "isInline" | "isMinimal"
    >
  >
>();

const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const buttonRef = ref<HTMLButtonElement | null>(null);
const dateInputRef = ref<HTMLInputElement | null>(null);
const timeInputRef = ref<HTMLInputElement | null>(null);
const width = computed(() => Math.max(MIN_WIDTH, props.size?.width ?? DEFAULT_WIDTH));

const currentMonth: Ref<DateTime> = ref(DateTime.now());
const dateString: Ref<string> = ref("");
const timeString: Ref<string> = ref("");
const unit = computed(() => {
  if (props.valueType?.primitiveType == PrimitiveType.DATE) {
    return "Date";
  } else if (props.valueType?.primitiveType == PrimitiveType.DATETIME) {
    return "Date/Time";
  } else if (props.valueType?.primitiveType == PrimitiveType.TIME) {
    return "Time";
  } else {
    return "Date/Time";
  }
});
const dateTime: Ref<DateTime | null> = computed(() => {
  const value = props.modelValue;
  if (!value) return null;

  if (unit.value == "Date") {
    return DateTime.fromObject({
      year: (value as ProtoDate).year,
      month: (value as ProtoDate).month,
      day: (value as ProtoDate).day,
    });
  } else if (unit.value == "Date/Time") {
    return tsToDt(value as Timestamp);
  } else {
    const dt = DateTime.now();
    return dt.set({
      hour: (value as TimeOfDay).hours,
      minute: (value as TimeOfDay).minutes,
    });
  }
});
// update from modelValue
watch(
  () => props.modelValue,
  () => {
    const dt = dateTime.value;
    if (!dt) {
      currentMonth.value = DateTime.now();
      dateString.value = "";
      timeString.value = "";
    } else {
      currentMonth.value = dt.startOf("month");
      dateString.value = formatInputString(dt);
      timeString.value = dt.toFormat("HH:mm");
    }
  },
  { immediate: true },
);

// weeks
const weeks = computed(() => {
  // period
  const start = currentMonth.value.startOf("month").startOf("week");
  const end = currentMonth.value.endOf("month").endOf("week");
  // days
  const days: DateTime[] = [];
  let current = start;
  while (current <= end) {
    days.push(current);
    current = current.plus({ days: 1 });
  }
  // group by week
  return days.reduce<DateTime[][]>((weeks, day) => {
    if (day.weekday === 1) weeks.push([]);
    weeks[weeks.length - 1].push(day);
    return weeks;
  }, []);
});
const weekdays = computed(() => {
  return [...Array(7)].map((_, i) =>
    DateTime.local()
      .set({ weekday: (i + 1) as WeekdayNumbers })
      .toFormat("ccc"),
  );
});

function formatInputString(dt: DateTime): string {
  if (unit.value === "Time") {
    return dt.toFormat("HH:mm");
  } else {
    return dt.toFormat("yyyy/MM/dd");
  }
}
function formatDisplayString(dt: DateTime): string {
  let displayString;
  if (unit.value == "Date") {
    displayString = dateString.value;
  } else if (unit.value == "Time") {
    displayString = timeString.value;
  } else if (unit.value == "Date/Time" && dateTime.value) {
    displayString = formatAbsoluteDate(props.modelValue as Timestamp);
  } else {
    displayString = "???";
  }
  return displayString;
}

function setDateString(string: string) {
  let parsedDateTime = DateTime.fromFormat(string, "yyyy/MM/dd");
  if (!parsedDateTime.isValid) return;
  if (unit.value == "Date/Time" && dateTime.value) {
    parsedDateTime = parsedDateTime.set({ hour: dateTime.value.hour, minute: dateTime.value.minute });
  }
  applyDateTime(parsedDateTime);
}

function setTimeString(string: string) {
  let parsedDateTime = DateTime.fromFormat(string, "HH:mm");
  if (!parsedDateTime.isValid) return;
  if (unit.value == "Date/Time" && dateTime.value) {
    parsedDateTime = parsedDateTime.set({
      year: dateTime.value.year,
      month: dateTime.value.month,
      day: dateTime.value.day,
    });
  }
  applyDateTime(parsedDateTime);
}

function goToNextPeriod() {
  currentMonth.value = currentMonth.value.plus({ months: 1 });
}
function goToPreviousPeriod() {
  currentMonth.value = currentMonth.value.minus({ months: 1 });
}

function isSelected(day: DateTime): boolean {
  if (!dateTime.value) {
    return false;
  } else if (unit.value == "Date") {
    return day.day === (props.modelValue as ProtoDate).day && day.month === dateTime.value.month;
  } else if (unit.value == "Date/Time") {
    return day.hasSame(dateTime.value, "day") && day.month === dateTime.value.month;
  } else {
    return day.hour === (props.modelValue as TimeOfDay).hours && day.minute === (props.modelValue as TimeOfDay).minutes;
  }
}
function isToday(day: DateTime): boolean {
  return day.hasSame(DateTime.now(), "day");
}
function isSamePeriod(day: DateTime): boolean {
  return day.hasSame(currentMonth.value, "month");
}

function selectDay(day: DateTime) {
  const time = DateTime.fromFormat(timeString.value || "00:00", "HH:mm");
  const selected = day.set({ hour: time.isValid ? time.hour : 0, minute: time.isValid ? time.minute : 0 });
  dateString.value = formatInputString(selected);
  applyDateTime(selected);
}

function applyDateTime(value: DateTime) {
  let modelValue: Timestamp | ProtoDate | TimeOfDay;
  if (unit.value == "Date") {
    modelValue = { year: value.year, month: value.month, day: value.day } as ProtoDate;
  } else if (unit.value == "Date/Time") {
    modelValue = dtToTs(value);
  } else {
    modelValue = { hours: value.hour, minutes: value.minute, seconds: 0 } as TimeOfDay;
  }
  apply(modelValue);
}

function apply(modelValue: Timestamp | ProtoDate | TimeOfDay) {
  emit("update:modelValue", modelValue);
  emit("apply", modelValue ?? undefined, true /* keep open */);
}

function clear() {
  emit("update:modelValue", undefined);
  dateString.value = "";
  timeString.value = "";
}

// Focus handling
function focus() {
  return buttonRef.value ?? dateInputRef.value ?? timeInputRef.value;
}

canvas.registerView(self, id);
defineExpose<ViewExposed>({
  self,
  id,
  focus,
  interact: () => buttonRef.value?.click(),
});
</script>

<template>
  <ViewContentWrapper :type="ViewType.DATETIME" v-bind="props">
    <!-- Dropdown Button -->
    <div
      v-if="!isInline"
      ref="buttonRef"
      v-menu="
        (): PopoverInfoIn => ({
          kind: 'view',
          component: ViewType.DATETIME,
          placement: 'inside-top-left',
          isEnabled: !isDisabled && isInput,
          offset: isMinimal ? { x: -10, y: -5 } : undefined,
          referenceMargin: 0,
          dontAnimate: isMinimal,
          props: {
            ...props,
            size: {
              metatype: ObjectType.RECTANGLE,
              width: Math.max(
                MIN_WIDTH,
                buttonRef?.getBoundingClientRect().width! + (isMinimal ? 10 : 0), // see above
              ),
            },
            title: undefined,
            isPopover: true,
            isInline: true,
          },
          onApply: (value: any) => emit('update:modelValue', value),
        })
      "
      role="button"
      :disabled="isDisabled || !isInput"
      class="group flex w-full flex-row items-center gap-x-1.5 rounded border-gray-200 hover:border-gray-200 data-[popover=true]:border-gray-200"
      :class="[!isMinimal ? 'border px-2.5 py-1' : '']"
    >
      <!-- Current value display -->
      <template v-if="modelValue">
        <IconInline
          v-if="icon || !isMinimal"
          v-bind="icon ?? ICON_BY_PRIMITIVE_TYPE[valueType?.primitiveType ?? PrimitiveType.DATETIME]"
          class="w-5 text-center text-gray-700"
        />
        <span class="truncate">{{ formatDisplayString(currentMonth) }}</span>
        <!-- Clear button -->
        <button
          v-if="!isDisabled && isInput && !valueType?.isRequired"
          class="ml-auto text-gray-400 opacity-0 transition-colors duration-150 hover:text-gray-700 group-hover:opacity-100"
          @click.stop="clear"
        >
          <i class="fas fa-xmark" />
        </button>
      </template>
      <!-- No value -->
      <span
        v-else
        class="text-gray-400 transition-colors duration-150"
        :class="isMinimal ? 'opacity-0 group-hover:opacity-100' : ''"
      >
        <IconInline
          v-if="icon || !isMinimal"
          v-bind="icon ?? ICON_BY_PRIMITIVE_TYPE[valueType?.primitiveType ?? PrimitiveType.DATETIME]"
          class="mr-1.5 w-5"
        />
        <span>Select {{ unit }}</span>
      </span>
    </div>

    <!-- Inline Picker -->
    <div v-else class="" :style="{ width: width + 'px' }">
      <div class="flex flex-col gap-2" :class="isPopover ? 'mx-2 mb-1 mt-2' : ''">
        <!-- Date/Time Input -->
        <div class="flex flex-row items-center gap-2">
          <!-- Date -->
          <input
            v-if="unit !== 'Time'"
            ref="dateInputRef"
            :value="dateString"
            type="text"
            class="min-w-0 flex-1 rounded border border-gray-200 bg-gray-100 px-2 py-1 text-sm outline-none ring-0 focus:border-gray-400 focus:ring-0"
            :placeholder="'YYYY/MM/DD'"
            @input="(e) => setDateString((e.target as HTMLInputElement).value)"
          />
          <!-- Time -->
          <input
            v-if="unit !== 'Date'"
            ref="timeInputRef"
            :value="timeString"
            type="text"
            class="min-w-0 flex-1 rounded border border-gray-200 bg-gray-100 px-2 py-1 text-sm outline-none ring-0 focus:border-gray-400 focus:ring-0"
            placeholder="HH:MM"
            @input="(e) => setTimeString((e.target as HTMLInputElement).value)"
          />
        </div>

        <!-- Calendar -->
        <div v-if="unit !== 'Time'" class="flex flex-col gap-2">
          <!-- Month Navigation -->
          <div class="mx-2 flex items-center justify-between">
            <button class="text-gray-600 hover:text-gray-900" @click="goToPreviousPeriod">
              <i class="fas fa-chevron-left" />
            </button>
            <button class="rounded px-1 font-medium hover:bg-gray-100" @click="currentMonth = DateTime.now()">
              {{ currentMonth.toFormat("LLLL yyyy") }}
            </button>
            <button class="text-gray-600 hover:text-gray-900" @click="goToNextPeriod">
              <i class="fas fa-chevron-right" />
            </button>
          </div>

          <!-- Weekday Headers -->
          <div class="grid grid-cols-7 text-center text-sm text-gray-500">
            <div v-for="day in weekdays" :key="day" class="h-8 leading-8">
              {{ day }}
            </div>
            <button
              v-for="day in weeks.flat()"
              :key="day.toISO()!"
              class="h-8 w-full rounded text-sm transition-colors duration-75"
              :class="[
                isSamePeriod(day) ? 'text-gray-900' : 'text-gray-400',
                isSelected(day) ? 'bg-gray-100 font-bold text-gray-900' : 'hover:bg-gray-100',
                isToday(day) ? 'font-bold' : '',
              ]"
              @click="selectDay(day)"
            >
              {{ day.toFormat("d") }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </ViewContentWrapper>
</template>
