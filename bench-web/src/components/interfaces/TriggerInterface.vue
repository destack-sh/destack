<script lang="ts" setup>
import Switch from "@/components/basic/Switch.vue";
import { TriggerType, type Trigger, ScheduleType } from "@/gql/graphql";
import {
  TRIGGER_ICONS_SOLID,
  useTriggerSchedule,
  type INTERVAL_UNIT,
  INTERVAL_UNITS,
  getTriggerIntervalUnit,
  VALID_INTERVAL_VALUES_BY_UNIT,
} from "@/state/trigger";
import { toCamelCase } from "@/utils/functools";
import { computed, ref, watch, toRef } from "vue";
import { DateTime } from "luxon";
import { Listbox, ListboxButton, ListboxOption, ListboxOptions } from "@headlessui/vue";

const props = defineProps<{ modelValue: Trigger; readonly: boolean }>();
const emit = defineEmits<{
  (e: "update:modelValue", value: Trigger): void;
  (e: "navigateDown"): void;
  (e: "navigateUp"): void;
}>();

const localeTimezone = DateTime.local().zoneName;
const timezone = computed(() => props.modelValue.timezone ?? "UTC");
const scheduleType = computed(() => props.modelValue.scheduleType ?? ScheduleType.Interval);
const cron = ref(props.modelValue.cron ?? "");
const schedule = useTriggerSchedule(
  computed(() => ({
    ...props.modelValue,
    cron: cron.value,
  }))
);

// sync model value cron into cron
watch(
  () => props.modelValue.cron,
  (value) => {
    cron.value = value ?? "";
  }
);

// update cron value if schedule is valid
watch(
  () => [cron.value],
  () => {
    if (schedule.value?.valid) {
      update({ cron: cron.value });
    }
  }
);

const DEFAULT_INTERVAL_BY_UNIT: Record<INTERVAL_UNIT, number> = {
  minute: 30,
  hour: 1,
  day: 1,
  week: 1,
};
const ABS_MIN_INTERVAL = 60;
const intervalDisplayUnit = ref<INTERVAL_UNIT>(getTriggerIntervalUnit(props.modelValue.interval ?? 60 * 60));
const intervalLimits = computed(() => VALID_INTERVAL_VALUES_BY_UNIT[intervalDisplayUnit.value]);
const lastIntervalByUnit: Partial<Record<INTERVAL_UNIT, number>> = {};

function setIntervalDisplayUnit(unit: INTERVAL_UNIT) {
  /*
  Sets the interval display unit and remembers the current value per unit to restore to (with a default) 
  Keep the interval display value if it's a valid value
  */
  if (unit == intervalDisplayUnit.value) return;
  const currentInterval = props.modelValue.interval ?? 1;
  lastIntervalByUnit[intervalDisplayUnit.value] = currentInterval;
  const nextLimits = VALID_INTERVAL_VALUES_BY_UNIT[unit];

  if (currentInterval < nextLimits.min || currentInterval > nextLimits.max) {
    const newInterval = lastIntervalByUnit[unit] ?? DEFAULT_INTERVAL_BY_UNIT[unit] * INTERVAL_UNITS[unit];
    update({ interval: newInterval });
  }
  intervalDisplayUnit.value = unit;
}

function setInterval(intervalInUnit: number) {
  const interval = Math.max(intervalInUnit * INTERVAL_UNITS[intervalDisplayUnit.value], ABS_MIN_INTERVAL);
  update({ interval });
}

function toggleScheduleType() {
  if (scheduleType.value == ScheduleType.Cron) {
    update({
      scheduleType: ScheduleType.Interval,
      interval: Math.max(ABS_MIN_INTERVAL, props.modelValue.interval ?? 60 * 60),
    });
  } else if (scheduleType.value == ScheduleType.Interval) {
    update({ scheduleType: ScheduleType.Cron });
  } else {
    throw new Error(`unexpected schedule type ${scheduleType.value}`);
  }
}

function update(properties: Partial<Trigger>) {
  emit("update:modelValue", { ...props.modelValue, ...properties });
}
</script>
<template>
  <div ref="containerRef" class="relative">
    <!-- Type & active -->
    <div class="flex flex-row items-center justify-between">
      <span
        class="inline-flex flex-row items-center"
        :class="[props.modelValue.active ? 'text-orange-600' : 'text-gray-700']"
      >
        <component :is="TRIGGER_ICONS_SOLID[props.modelValue.type]" class="mr-1 h-4 w-4" />
        <span class="font-bold">{{ toCamelCase(props.modelValue.type) }} trigger</span>
        <span v-if="!props.modelValue.active" class="ml-1 text-gray-400">(inactive)</span>
      </span>
      <div class="flex flex-row items-center gap-1.5">
        <slot name="actions" />
        <Switch :model-value="props.modelValue.active" @update:model-value="update({ active: $event })" />
      </div>
    </div>
    <!-- Trigger details -->
    <div v-if="props.modelValue.type == TriggerType.Time" class="mt-1.5 flex flex-col">
      <!-- Interval/cron -->
      <div class="flex flex-row items-center gap-2">
        <!-- Type -->
        <button
          class="inline-flex flex-row items-center rounded-sm border border-orange-900/[12%] px-2 py-1 hover:bg-orange-100 focus:outline-none focus:ring-0"
          @click="toggleScheduleType()"
        >
          <span class="text-gray-900">
            {{ scheduleType == ScheduleType.Interval ? "Every" : "Cron" }}
          </span>
        </button>
        <!-- Editable schedule -->
        <template v-if="scheduleType == ScheduleType.Interval">
          <!-- Interval -->
          <input
            ref="intervalInputRef"
            type="number"
            min="1"
            max="60"
            class="w-full max-w-full flex-grow scroll-m-0 overflow-x-hidden rounded-sm border border-orange-900/[12%] p-1 px-1 text-right text-sm font-bold text-gray-900 focus:border-orange-200 focus:bg-orange-100 focus:outline-none focus:ring-0"
            :value="(props.modelValue.interval ?? 0) / INTERVAL_UNITS[intervalDisplayUnit]"
            @input="setInterval(Number.parseInt(($event.target as HTMLInputElement)?.value))"
          />
          <!-- Interval unit -->
          <Listbox
            as="div"
            class="relative w-full"
            :model-value="intervalDisplayUnit"
            @update:model-value="setIntervalDisplayUnit"
          >
            <ListboxButton
              ref="intervalUnitRef"
              class="w-full flex-grow rounded-sm border border-orange-900/[12%] px-2 py-1 text-left font-bold text-gray-900 hover:bg-orange-100"
            >
              {{ intervalDisplayUnit }}s
            </ListboxButton>
            <ListboxOptions
              class="absolute left-0 top-8 z-10 mt-0 w-full rounded-sm bg-white px-1 py-1 shadow-md outline-none ring-1 ring-orange-900 ring-opacity-40"
            >
              <ListboxOption
                as="div"
                v-for="unit in Object.keys(INTERVAL_UNITS)"
                :key="unit"
                :value="unit"
                class="px-2 py-1 hover:bg-orange-100 focus:bg-orange-100"
                :class="[unit == intervalDisplayUnit ? 'text-orange-600' : '']"
              >
                {{ unit }}s
              </ListboxOption>
            </ListboxOptions>
          </Listbox>
        </template>
        <template v-else>
          <!-- Cron -->
          <input
            ref="cronInputRef"
            type="text"
            class="w-full max-w-full scroll-m-0 overflow-x-hidden rounded-sm border border-orange-900/[12%] p-1 px-1 text-center text-sm font-bold text-gray-900 focus:border-orange-200 focus:bg-orange-100 focus:outline-none focus:ring-0"
            v-model="cron"
          />
        </template>
        <!-- Timezone -->
        <span class="rounded-sm border border-orange-900/[12%] px-2 py-1 text-gray-900">UTC</span>
      </div>
      <!-- Human readable -->
      <div class="mt-1.5 px-1 py-1 text-center" v-if="scheduleType == ScheduleType.Cron">
        <span v-if="schedule?.valid" class="text-sm font-bold italic text-gray-900">"{{ schedule.humanized }}"</span>
        <span v-else-if="schedule?.valid === false" class="text-sm font-bold text-red-600">invalid cron</span>
        <span v-else-if="schedule?.valid === null" class="text-sm text-gray-600">parsing...</span>
      </div>
      <!-- Occurrences -->
      <div class="mt-2 w-full" v-if="schedule?.valid">
        <div class="flex flex-col gap-1 py-0.5 align-top text-sm">
          <!-- Previous -->
          <div class="flex flex-row justify-between gap-2 px-1 py-0.5">
            <span class="h-fit rounded-xl bg-stone-50 px-2 text-stone-700 ring-1 ring-inset ring-stone-500/20">
              last
            </span>
            <!-- Time (with relevant timezones) -->
            <span class="flex flex-col gap-0.5">
              <span class="text-gray-700">
                {{ schedule?.lastOccurrence?.setZone(timezone).toLocaleString(DateTime.DATETIME_FULL_WITH_SECONDS) }}
              </span>
              <!-- Other timezones (if different) -->
              <span
                v-for="tz in [timezone, localeTimezone, 'UTC'].filter((tz) => timezone != tz)"
                :key="tz"
                class="text-right text-xs text-gray-400"
              >
                {{ schedule?.lastOccurrence?.setZone(tz).toLocaleString(DateTime.DATETIME_FULL_WITH_SECONDS) }}
              </span>
            </span>
          </div>
          <!-- Next -->
          <div
            v-for="(occurrence, i) in schedule?.nextOccurrences"
            :key="i"
            class="flex w-full flex-row items-start justify-between gap-2 px-1 py-0.5"
          >
            <span
              class="rounded-xl"
              :class="[
                i == 0 && props.modelValue.active
                  ? 'bg-orange-50 px-2 text-orange-700 ring-1 ring-inset ring-orange-700/10'
                  : 'bg-stone-50 px-2 text-stone-700 ring-1 ring-inset ring-stone-500/20',
              ]"
            >
              {{ i == 0 ? "next" : "then" }}
            </span>
            <!-- Time (with UTC) -->
            <!-- Other timezones (if different) -->
            <span class="flex flex-col gap-0.5">
              <span
                :class="[i == 0 ? (props.modelValue.active ? 'text-orange-600' : 'text-gray-900') : 'text-gray-700']"
              >
                {{ occurrence.setZone(timezone).toLocaleString(DateTime.DATETIME_FULL_WITH_SECONDS) }}
              </span>
              <!-- Other timezones (if different) -->
              <span
                v-for="tz in [timezone, localeTimezone, 'UTC'].filter((tz) => timezone != tz)"
                :key="tz"
                class="text-right text-xs text-gray-400"
              >
                {{ occurrence.setZone(tz).toLocaleString(DateTime.DATETIME_FULL_WITH_SECONDS) }}
              </span>
            </span>
          </div>
        </div>
      </div>
    </div>
    <div v-else>
      <!-- show error? -->
    </div>
  </div>
</template>
