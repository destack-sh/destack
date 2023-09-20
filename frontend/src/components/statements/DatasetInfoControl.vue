<script lang="ts" setup>
import type { StatementEmit, StatementProps } from "@/components/statements";
import { useOperations } from "@/state/operations";
import { ClockIcon, GlobeAltIcon } from "@heroicons/vue/24/outline";
import { computed, ref } from "vue";
import { DATASET_VERSIONED_RECORD_LIMIT } from "@/state/module";
import { useNotifications } from "@/state/notifications";

const props = defineProps<Pick<StatementProps, "statement" | "focused" | "readonly">>();
const emit = defineEmits<StatementEmit>();

const ops = useOperations();
const notifications = useNotifications();

const infoButtonRef = ref<HTMLButtonElement | null>(null);

const actions = computed(() => [
  {
    label: props.statement.versioned ? "Make global" : "Make local",
    groupId: "edit",
    icon: GlobeAltIcon,
    disabled: props.readonly,
    action: () => {
      // TODO @UX: confirm before making dataset global/local?
      //  (maybe add general confirm option to actions)
      // TODO @UX @Robustness: prevent morph to versioned if record count is too large
      // TODO @UX: localizing dataset does not actually copy it
      ops.statement.morph(null, props.statement.id, props.statement, {
        ...props.statement,
        versioned: !props.statement.versioned,
      });
    },
    hideInline: true,
  },
]);
defineExpose({
  focus: (f: "first" | "last" = "first") => {
    infoButtonRef.value?.focus();
  },
  blur: () => {
    infoButtonRef.value?.blur();
  },
  actions,
});
</script>
<template>
  <div>
    <!-- For now just info button -->
    <button
      ref="infoButtonRef"
      class="group-hover:statement/text-gray-400 group flex flex-row rounded-sm transition duration-150 hover:bg-orange-100 focus:bg-orange-100 focus:text-gray-700 focus:outline-none"
      :class="[focused ? 'text-gray-400' : 'text-gray-300']"
      @click="emit('openActions')"
      @keydown.left.exact.prevent="emit('navigateLeft')"
      @keydown.right.exact.prevent="emit('navigateRight')"
      @keydown.up.exact.prevent="emit('navigateUp')"
      @keydown.down.exact.prevent="emit('navigateDown')"
    >
      <component :is="statement.versioned ? ClockIcon : GlobeAltIcon" class="mr-0.5 mt-0.5 h-4 w-4" />
      <!-- Label popover -->
      <span
        v-if="!readonly"
        class="pointer-events-none absolute left-3 top-6 z-10 whitespace-nowrap rounded-sm border border-orange-900 border-opacity-[15%] bg-white px-2 py-0.5 text-center text-xs text-gray-700 opacity-0 transition delay-in-500 duration-150 group-hover:opacity-100 group-focus:opacity-100"
      >
        {{
          statement.versioned
            ? "Local dataset, versioned with this Bench"
            : "Global dataset, not versioned with this Bench"
        }}
      </span>
    </button>
  </div>
</template>
