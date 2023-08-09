<script lang="ts" setup>
import Switch from "@/components/basic/Switch.vue";
import { TriggerType, type Trigger, ScheduleType } from "@/gql/graphql";
import { TRIGGER_ICONS_SOLID } from "@/state/statement";
import { toCamelCase } from "@/utils/functools";
import { ArrowPathRoundedSquareIcon, StarIcon } from "@heroicons/vue/24/outline";
import { computed, ref, watch, type Ref } from "vue";
import cronInterval from "cron-parser";
import cronHunaize from "cronstrue";
import { syncProperty } from "@/utils/sync";
import { DateTime } from "luxon";
import { useTimeFromNow } from "@/composables/useNow";

const INTERVAL_UNITS = {
  minute: 60,
  hour: 60 * 60,
  day: 60 * 60 * 24,
  week: 60 * 60 * 24 * 7,
};
type INTERVAL_UNIT = keyof typeof INTERVAL_UNITS;
const OCCURENCES_PREVIEW = 2;

const props = defineProps<{ modelValue: Trigger; readonly: boolean }>();
const emit = defineEmits<{ (e: "update:modelValue", value: Trigger): void }>();

const now = useTimeFromNow(1000);

const scheduleType = computed(() => props.modelValue.scheduleType);
const cron = ref(props.modelValue.cron ?? "");
const intervalUnit = ref<INTERVAL_UNIT>("minute");
const valid: Ref<boolean | null> = ref(null);
const humanized: Ref<string> = ref("");
const lastOccurence: Ref<DateTime | null> = ref(null);
const nextOccurences: Ref<DateTime[]> = ref([]);

// sync model value cron into cron
watch(
  () => props.modelValue.cron,
  (value) => {
    cron.value = value ?? "";
  }
);

// sync interval unit
watch(
  () => props.modelValue.interval,
  () => {
    // get first round match
    const unit = Object.entries(INTERVAL_UNITS).find(([, v]) => (props.modelValue.interval ?? 60 * 60) % v == 0);
    if (unit != null) {
      intervalUnit.value = unit[0] as INTERVAL_UNIT;
    }
  },
  {
    immediate: true,
  }
);

// update validity / intervals / etc.
watch(
  () => [scheduleType.value, cron.value, props.modelValue.interval, now.now.value],
  () => {
    console.log("update validity");
    valid.value = false;

    if (scheduleType.value == ScheduleType.Interval) {
      valid.value = true;
      humanized.value = `Every ${props.modelValue.interval} ${intervalUnit.value}`;
      nextOccurences.value = [];
      for (let i = 0; i < OCCURENCES_PREVIEW; i++) {
        nextOccurences.value.push(now.now.value.plus({ [intervalUnit.value]: props.modelValue.interval }));
      }
      lastOccurence.value = now.now.value.minus({ [intervalUnit.value]: props.modelValue.interval });
    } else {
      try {
        const interval = cronInterval.parseExpression(cron.value);
        valid.value = true;
        humanized.value = cronHunaize.toString(cron.value);

        lastOccurence.value = DateTime.fromMillis(interval.prev().getTime());
        nextOccurences.value = [];
        for (let i = 0; i < OCCURENCES_PREVIEW; i++) {
          const next = interval.next();
          if (next == null) break;
          nextOccurences.value.push(DateTime.fromMillis(next.getTime()));
        }
      } catch (e) {
        // nothing to do
      }
    }
  }
);

function toggleScheduleType() {
  if (scheduleType.value == ScheduleType.Cron) {
    update({ scheduleType: ScheduleType.Interval });
  } else {
    update({ scheduleType: ScheduleType.Cron });
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
      <span class="inline-flex flex-row items-center">
        <component :is="TRIGGER_ICONS_SOLID[props.modelValue.type]" class="mr-1 h-4 w-4 text-orange-900" />
        <span class="font-bold text-orange-900">{{ toCamelCase(props.modelValue.type) }} trigger</span>
      </span>
      <Switch :model-value="props.modelValue.active" @update:model-value="update({ active: $event })" />
    </div>
    <!-- Trigger details -->
    <div v-if="props.modelValue.type == TriggerType.Time" class="mt-1.5 flex flex-col">
      <!-- Interval/cron -->
      <div class="flex flex-row items-center gap-2">
        <!-- Type -->
        <button
          class="inline-flex flex-row items-center rounded-sm border border-orange-900 border-opacity-[12%] p-1 hover:bg-orange-100 focus:outline-none focus:ring-0"
          @click="toggleScheduleType()"
        >
          <component
            :is="scheduleType == ScheduleType.Interval ? ArrowPathRoundedSquareIcon : StarIcon"
            class="mr-1 h-4 w-4 text-gray-400"
          />
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
            class="w-full max-w-full scroll-m-0 overflow-x-hidden rounded-sm border border-orange-900 border-opacity-[12%] p-1 text-sm text-gray-900 focus:bg-orange-100 focus:bg-orange-100 focus:outline-none focus:ring-0"
          />
          <span ref="intervalUnitRef" class="text-gray-900">minutes</span>
        </template>
        <template v-else>
          <!-- Cron -->
          <input
            ref="cronInputRef"
            type="text"
            class="w-full max-w-full scroll-m-0 overflow-x-hidden rounded-sm border border-orange-900 border-opacity-[12%] p-1 text-sm text-gray-900 focus:bg-orange-100 focus:bg-orange-100 focus:outline-none focus:ring-0"
            v-model="cron"
          />
        </template>
      </div>
      <!-- Human readable -->
      <div class="mt-1.5 text-center">
        <span v-if="valid" class="text-sm font-bold italic text-gray-900">"{{ humanized }}"</span>
        <span v-else-if="valid === false" class="text-sm font-bold text-red-600">invalid cron</span>
        <span v-else-if="valid === null" class="text-sm text-gray-600">parsing...</span>
      </div>
      <!-- Next occurrences of tirgger  -->
      <div class="mt-1 py-0.5" v-if="valid">
        <span class="text-xs uppercase text-gray-400"
          >next up <template v-if="!props.modelValue.active">(inactive)</template></span
        >
        <div class="flex flex-col text-sm">
          <div v-for="(occurence, i) in [lastOccurence, ...nextOccurences]" :key="i">
            {{ occurence }}
          </div>
        </div>
      </div>
    </div>
    <div v-else>
      <!-- show error? -->
    </div>
  </div>
</template>
