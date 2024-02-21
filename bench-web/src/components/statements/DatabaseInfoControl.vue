<script lang="ts" setup>
import type { StatementEmit, StatementProps } from "@/components/statements";
import { useOperations } from "@/state/operations";
import { ClockIcon as ClockIconOutline, GlobeAltIcon as GlobeAltIconOutline } from "@heroicons/vue/24/outline";
import { GlobeAltIcon as GlobeAltIconSolid } from "@heroicons/vue/24/solid";
import { computed, ref } from "vue";

const props = defineProps<Pick<StatementProps, "statement" | "focused" | "readonly">>();
const emit = defineEmits<StatementEmit>();

const ops = useOperations();

const infoButtonRef = ref<HTMLButtonElement | null>(null);

const actions = computed(() => [
  {
    label: props.statement.versioned ? "Promote to global" : "Turn into inline",
    groupId: "edit",
    icon: GlobeAltIconOutline,
    disabled: props.readonly,
    action: () => {
      // TODO @UX: confirm before making database global/local?
      //  (maybe add general confirm option to actions)
      // TODO @UX :Robustness: prevent morph to versioned if record count is too large
      // TODO @UX: localizing database after global does not actually copy it
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
    <!-- Info button -->
    <button
      ref="infoButtonRef"
      class="group flex flex-row rounded-sm focus:outline-none"
      :class="[
        statement.versioned && focused ? 'text-gray-400' : '',
        statement.versioned && !focused ? 'text-gray-300' : '',
        statement.versioned
          ? 'group-hover:statement/text-gray-400 transition duration-150 hover:bg-orange-100 focus:bg-orange-100 focus:text-gray-700'
          : 'text-emerald-900  hover:bg-emerald-100 focus:bg-emerald-100',
      ]"
      @click="emit('openActions')"
      @keydown.left.exact.prevent="emit('navigateLeft')"
      @keydown.right.exact.prevent="emit('navigateRight')"
      @keydown.up.exact.prevent="emit('navigateUp')"
      @keydown.down.exact.prevent="emit('navigateDown')"
    >
      <component :is="statement.versioned ? ClockIconOutline : GlobeAltIconSolid" class="mr-0.5 mt-0.5 h-4 w-4" />
      <!-- Label popover -->
      <span
        v-if="!readonly"
        class="pointer-events-none absolute left-3 top-6 z-10 whitespace-nowrap rounded-sm border border-orange-900 border-opacity-[15%] bg-white px-2 py-0.5 text-center text-xs text-gray-500 text-gray-700 opacity-0 transition delay-in-500 duration-150 group-hover:opacity-100 group-focus:opacity-100"
      >
        <span class="font-semibold">{{ statement.versioned ? "Inline" : "Global" }} database</span>:
        {{ statement.versioned ? "records are tied to Bench version" : "records are shared across Bench versions" }}
      </span>
    </button>
  </div>
</template>
