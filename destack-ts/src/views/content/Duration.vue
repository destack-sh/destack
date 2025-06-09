<script lang="ts" setup>
import { NodeType, ObjectType, RectangleData, ViewData, ViewType } from "@/proto/wire";
import { Duration as ProtoDuration } from "@/proto/wire/google/protobuf/duration";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import { IconInline } from "@/ui/icon";
import type { PopoverInfoIn } from "@/ui/popover";
import { formatDuration, parseDurationString } from "@/utils/time";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import { Duration } from "luxon";
import { computed, ref, toRef, watch } from "vue";

const MIN_WIDTH = 280;
const DEFAULT_WIDTH = 400;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    size?: Partial<Pick<RectangleData, "width" | "height">>;
    modelValue?: ProtoDuration;
    isPopover?: boolean;
  } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "isInput" | "isDisabled" | "isInline" | "isMinimal">
  >
>();

const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
canvas.registerView(self, id);

const durationString = ref("");
const width = computed(() => Math.max(MIN_WIDTH, props.size?.width ?? DEFAULT_WIDTH));
const buttonRef = ref<HTMLButtonElement | null>(null);
const queryRef = ref<HTMLInputElement | null>(null);

const duration = computed(() => {
  if (!props.modelValue) return null;
  return Duration.fromMillis(Number(props.modelValue.seconds) * 1000 + Number(props.modelValue.nanos) / 1_000_000);
});

function toProtoDuration(dur: Duration): ProtoDuration {
  const totalMillis = dur.toMillis();
  const seconds = Math.floor(totalMillis / 1000);
  const nanos = (totalMillis % 1000) * 1_000_000;
  return { seconds: BigInt(seconds), nanos: nanos };
}

// quick presets
const presets = [
  { label: "2s", duration: Duration.fromObject({ seconds: 2 }) },
  { label: "1m30s", duration: Duration.fromObject({ minutes: 1, seconds: 30 }) },
  { label: "30m", duration: Duration.fromObject({ minutes: 30 }) },
  { label: "2h", duration: Duration.fromObject({ hours: 2 }) },
  { label: "1d8h", duration: Duration.fromObject({ days: 1, hours: 2 }) },
] as const;

// update input when modelValue changes
watch(
  () => props.modelValue,
  () => {
    const dur = duration.value;
    if (!dur) {
      durationString.value = "";
    } else {
      durationString.value = formatDuration(dur, { format: "short", digits: 0, extended: true });
    }
  },
  { immediate: true },
);

function apply(value: ProtoDuration | undefined, keepOpen = false) {
  emit("update:modelValue", value);
  emit("apply", value, keepOpen);
}

function clear() {
  apply(undefined);
  durationString.value = "";
}

function setDurationString(string: string) {
  const dur = parseDurationString(string);
  if (!dur) return;
  const newModelValue = toProtoDuration(dur);
  if (
    !props.modelValue ||
    props.modelValue.seconds !== newModelValue.seconds ||
    props.modelValue.nanos !== newModelValue.nanos
  ) {
    apply(newModelValue, true);
  }
}

function selectPreset(preset: (typeof presets)[number]) {
  apply(toProtoDuration(preset.duration), true);
  durationString.value = preset.label;
}

function focus() {
  return buttonRef.value ?? queryRef.value;
}

defineExpose<ViewExpose>({
  self,
  id,
  focus,
  interact: () => buttonRef.value?.click(),
});
</script>
<template>
  <div
    v-if="!isInline"
    ref="buttonRef"
    v-menu="
      (): PopoverInfoIn => ({
        kind: 'view',
        component: ViewType.DURATION,
        placement: 'inside-top-left',
        isEnabled: !isDisabled && isInput,
        offset: isMinimal ? { x: -10, y: -45 } : { x: 0, y: -40 },
        referenceMargin: 0,
        dontAnimate: isMinimal,
        props: {
          ...props,
          size: {
            metatype: ObjectType.RECTANGLE,
            width: Math.max(MIN_WIDTH, buttonRef?.getBoundingClientRect().width! + (isMinimal ? 10 : 0)),
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
    class="group flex w-full flex-row items-center gap-x-1.5 rounded-sm border-gray-200 hover:border-gray-200 data-[popover=true]:border-gray-200"
    :class="[!isMinimal ? 'border px-2 py-1' : '']"
  >
    <!-- Dropdown -->
    <!-- Current value display -->
    <template v-if="modelValue">
      <IconInline
        v-if="icon || !isMinimal"
        v-bind="icon ?? { faName: 'fas fa-clock' }"
        class="w-5 text-center group-hover:text-gray-700"
        :class="durationString ? 'text-gray-700' : 'text-gray-400'"
      />
      <span v-if="duration" class="truncate">{{ formatDuration(duration, { format: "regular" }) }}</span>
      <!-- Clear button -->
      <button
        v-if="!isDisabled && isInput"
        class="ml-auto cursor-pointer text-gray-400 opacity-0 transition-colors duration-75 group-hover:opacity-100 hover:text-gray-700"
        @click.stop="clear"
      >
        <i class="fas fa-xmark" />
      </button>
    </template>
    <!-- No value -->
    <span
      v-else
      class="text-gray-400 transition-colors duration-75 group-hover:text-gray-700"
      :class="isMinimal ? 'opacity-0 group-hover:opacity-100' : ''"
    >
      <IconInline v-if="icon || !isMinimal" v-bind="icon ?? { faName: 'fas fa-clock' }" class="mr-1.5 w-5" />
      <span>Select Duration</span>
    </span>
  </div>

  <div v-else class="flex flex-col gap-2" :style="{ width: width + 'px' }">
    <!-- Inline Editor -->
    <div class="flex flex-col gap-2" :class="isPopover ? 'mx-2 mt-2 mb-1' : ''">
      <!-- Quick presets -->
      <div class="flex flex-wrap gap-1">
        <button
          v-for="preset in presets"
          :key="preset.label"
          class="cursor-pointer rounded-sm bg-gray-100 px-2 py-0.5 text-sm text-gray-600 hover:bg-gray-200"
          @click="selectPreset(preset)"
        >
          {{ preset.label }}
        </button>
      </div>

      <!-- Duration input -->
      <input
        ref="queryRef"
        :value="durationString"
        type="text"
        class="w-full rounded-sm border border-gray-200 bg-gray-100 px-2 py-1 text-sm ring-0 outline-hidden focus:border-gray-400 focus:ring-0"
        placeholder="e.g. 1h 30m, 2d, 1y"
        @input="(e) => setDurationString((e.target as HTMLInputElement).value)"
      />

      <!-- Preview -->
      <div v-if="duration" class="text-center text-sm text-gray-400 italic">
        "{{ formatDuration(duration, { format: "long" }) }}"
      </div>
    </div>
  </div>
</template>
