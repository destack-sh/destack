<script lang="ts" setup>
import AnnotatedText from "@/components/interfaces/AnnotatedText.vue";
import type { StatementEmit, StatementProps } from "@/components/statements";
import { useElementRefs } from "@/composables/useGrid";
import { StatementType } from "@/gql/graphql";
import { useOperations } from "@/state/operations";
import { getStatementIconSolid } from "@/state/statement";
import { syncProperty } from "@/utils/sync";
import { AtSymbolIcon } from "@heroicons/vue/24/solid";
import { computed, nextTick, ref, watch, type Ref, type Component } from "vue";

const props = defineProps<Pick<StatementProps, "statement" | "readonly" | "focused" | "editing">>();
const emit = defineEmits<StatementEmit>();

const ops = useOperations();
const textRef: Ref<InstanceType<typeof AnnotatedText> | null> = ref(null);
const text: Ref<string> = ref(props.statement.text ?? "");
const textSync = syncProperty({
  value: text,
  editing: computed(() => textRef.value?.focused),
  read: () => (text.value = props.statement.text ?? ""),
  write: () => ops.symbol.updateStatementText(null, props.statement.id, props.statement.text ?? "", text.value),
  debounceMs: 500,
  debounceMaxWait: 3000,
});

function focus(position: "first" | "last" = "first") {
  // focus the end of the content if we just updated it, which puts it in pending state
  // (likely due to a morph to blank where we want to keep editing smoothly)
  const focusEnd = props.statement.revision < 0 || position == "last";
  // not sure why we need both, but acquiring focus doesn't always succeed otherwise
  textRef.value?.focus(focusEnd ? "last" : "first"); // maybe we should always focus last here?
}

function blur() {
  textRef.value?.blur();
  quickActionsRefs.refs.value.forEach((ref) => ref?.blur());
}

// if text statement: morph to blank if empty
watch(
  text,
  () => {
    if (props.statement.type == StatementType.Text && (props.statement.headingLevel ?? 0) == 0) {
      if (text.value == "") {
        // should ideally be done in one tx, but we don't have that yet
        textSync.flushNow();
        ops.statement.morph(null, props.statement.id, props.statement, {
          type: StatementType.Blank,
          headingLevel: null,
        });
      }
    }
  },
  { immediate: true }
);

// quick inline actions
type QuickAction = {
  id: string;
  icon: Component;
  action: () => void;
};
const quickActions = computed(() => {
  if (
    props.statement.type != StatementType.Text ||
    (props.statement.headingLevel ?? 0) != 0 ||
    props.statement.name != null ||
    props.readonly
  )
    return [];
  const actions: QuickAction[] = [
    {
      id: "task",
      icon: getStatementIconSolid(StatementType.Task),
      action: () => ops.statement.morph(null, props.statement.id, props.statement, { type: StatementType.Task }),
    },
  ];
  if (props.statement.name == null) {
    actions.push({
      id: "name",
      icon: AtSymbolIcon,
      action: () => {
        ops.statement.rename(null, props.statement.id, null, "");
        nextTick(() => emit("focus", "declaration"));
      },
    });
  }
  actions.push({
    id: "code",
    icon: getStatementIconSolid(StatementType.Code),
    action: () => {
      ops.statement.morph(null, props.statement.id, props.statement, { type: StatementType.Code });
      nextTick(() => emit("focus", "code"));
    },
  });
  actions.push({
    id: "data",
    icon: getStatementIconSolid(StatementType.Dataset),
    action: () => {
      ops.statement.morph(null, props.statement.id, props.statement, { type: StatementType.Dataset });
      nextTick(() => emit("focus", "dataset"));
    },
  });

  return actions;
});
const quickActionsRefs = useElementRefs<HTMLButtonElement>(quickActions, {
  navigateLeft: () => textRef.value?.focus("last"),
  navigateRight: () => focusAction(0),
});

function focusAction(id: string | number) {
  quickActionsRefs.focus(id);
}

defineExpose({
  focus,
  blur,
  syncNow: () => textSync.flushNow(),
});
</script>
<template>
  <div class="relative w-full text-gray-900" @click="textRef?.focus">
    <!-- Actual text -->
    <AnnotatedText
      ref="textRef"
      :model-value="text || ''"
      @update:model-value="text = $event"
      @navigate-up="emit('navigateUp')"
      @navigate-down="emit('navigateDown')"
      @navigate-left="emit('navigateLeft')"
      @navigate-right="quickActions.length > 0 ? quickActionsRefs.focus(quickActions[0].id) : emit('navigateRight')"
      @enter-left="emit('enterLeft')"
      @enter="emit('enter')"
      @enter-right="emit('enterRight')"
      @delete-left="emit('deleteLeft')"
      @delete-if-empty="emit('deleteSelf')"
      @paste="emit('paste')"
      :focused="focused"
      :readonly="readonly"
    />
    <!-- Placeholder if empty -->
    <template v-if="text.length == 0">&nbsp;</template>
    <button
      v-if="text.length == 0"
      class="absolute left-0 top-0 -m-0.5 -mx-0.5 flex flex-row items-center rounded-sm p-0.5 transition-colors duration-75 hover:bg-orange-100"
      :class="[focused ? 'text-gray-400' : 'text-gray-300']"
      @click="textRef?.focus"
    >
      Enter text...
    </button>
    <!-- Quick inline actions (positioned as not to disturb the flow) -->
    <!-- TODO @UX: inline actions don't wrap properly when text overflows -->
    <!-- :InlineButtonPillStyle -->
    <div v-if="quickActions.length > 0 && focused && editing" class="relative inline-block">
      <div class="absolute -top-4 ml-3 flex animate-fadeInSlow flex-row gap-2 whitespace-nowrap transition-opacity">
        <button
          v-for="action in quickActions"
          :key="action.id"
          :ref="(ref: any) => quickActionsRefs.registerRef(action.id, ref)"
          class="group flex h-fit max-h-fit flex-row items-center rounded-sm bg-orange-100 bg-opacity-10 px-1.5 text-gray-400 shadow-sm ring-1 ring-inset ring-yellow-600/20 transition-colors duration-150 hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 focus:text-gray-700 focus:outline-none"
          @click="action.action"
          @keydown.right.stop.prevent="quickActionsRefs.navigateRight(action.id)"
          @keydown.left.stop.prevent="quickActionsRefs.navigateLeft(action.id)"
        >
          <component
            :is="action.icon"
            class="mr-0.5 mt-0.5 h-4 w-4 text-gray-300 transition-colors duration-150 group-hover:text-gray-500 group-focus:text-gray-500"
          />
          <span>{{ action.id }}</span>
        </button>
      </div>
    </div>
  </div>
</template>
<style>
@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

.fade-in-slow {
  animation: fadeIn 500ms forwards;
  animation-delay: 1s;
}

.fade-out-fast:hover {
  opacity: 0;
  transition: opacity 150ms;
}
</style>
