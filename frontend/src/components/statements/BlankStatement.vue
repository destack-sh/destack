<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import EditableSpan from "@/components/basic/EditableSpan.vue";
import { TypeTag } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { Combobox, ComboboxOption, ComboboxInput, ComboboxOptions, ComboboxButton } from "@headlessui/vue";
import { useFocus, type MaybeElementRef } from "@vueuse/core";
import { computed, nextTick, ref, watch, type Ref } from "vue";
import { StatementType } from "@/gql/graphql";
import {
  getStatementLabel,
  getStatementDescription,
  getStatementIconSolid,
  useStatementContext,
} from "@/state/statement";
import { EllipsisHorizontalIcon } from "@heroicons/vue/24/outline";
import { useActiveScroll } from "@/composables/useScroll";

defineProps<{ showDots?: boolean; folded?: boolean }>();
const emit = defineEmits<{
  (e: "navigateUp", position?: number): void;
  (e: "navigateDown", position?: number): void;
  (e: "navigateLeft"): void;
  (e: "navigateRight"): void;
  (e: "enter"): void;
  (e: "escape"): void;
  (e: "deleteLeft"): void;
  (e: "morphed"): void;
}>();

const context = useStatementContext();
const query: Ref<string> = ref("");
const spanRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);

// open/close commanding and auto-convert to text on anything else
watch(query, (query) => {
  if (query.trim() == "") {
    commanding.value = false;
  } else if (query == "/") {
    openCommandSelection();
    return;
  } else if (!commanding.value) {
    context.morphToComment(query);
    emit("morphed");
  }
});

// command selection dropdown
// (not sure if this is the best place to put it)
const commanding: Ref<boolean> = ref(false);
const commandQuery: Ref<string> = ref("");
const commandInputRef: Ref<InstanceType<typeof ComboboxInput> | null> = ref(null);
const commandButtonRef: Ref<InstanceType<typeof ComboboxButton> | null> = ref(null);
const commandOptionsRef: Ref<InstanceType<typeof ComboboxOptions> | null> = ref(null);
const { focused: commandInputRefFocused } = useFocus(commandInputRef as MaybeElementRef);

useActiveScroll(computed(() => commandOptionsRef.value?.$el));

type Group = {
  name: string;
};

const GROUPS = {
  BASIC: { name: "Basic statements" },
  ADVANCED: { name: "Advanced statements" },
};

type Command = {
  group: Group;
  label: string;
  icon: any;
  description: string;
  action: () => void;
};

function singleStatementCommand(
  group: Group,
  type: StatementType,
  options?: { rootTypeTag?: TypeTag; icon?: any; label?: string; description?: string }
): Command {
  return {
    group,
    label: options?.label ?? getStatementLabel(type, options?.rootTypeTag),
    icon: options?.icon ?? getStatementIconSolid(type, options?.rootTypeTag),
    description: options?.description ?? getStatementDescription(type, options?.rootTypeTag),
    action: () => (context.morpthToSymbol({ type, rootTypeTag: options?.rootTypeTag }), emit("morphed")),
  };
}

// TODO @UX: blank statement menu sucks
const commands = computed(() => {
  const commands: Command[] = [
    // basic statements
    {
      group: GROUPS.BASIC,
      label: "Text",
      icon: getStatementIconSolid(StatementType.Text),
      description: "Just type for a markdown comment",
      action: () => ((query.value = ""), nextTick(() => spanRef.value?.focus())),
    },
    singleStatementCommand(GROUPS.BASIC, StatementType.Type, { rootTypeTag: TypeTag.Struct }),
    singleStatementCommand(GROUPS.BASIC, StatementType.Type, { rootTypeTag: TypeTag.Enum }),
    singleStatementCommand(GROUPS.BASIC, StatementType.Dataset),
    singleStatementCommand(GROUPS.BASIC, StatementType.Code),
    singleStatementCommand(GROUPS.BASIC, StatementType.Task),
    singleStatementCommand(GROUPS.BASIC, StatementType.Expectation),

    // advanced statements
    singleStatementCommand(GROUPS.ADVANCED, StatementType.Value),
    singleStatementCommand(GROUPS.ADVANCED, StatementType.Flow),
  ];

  return commands;
});

const filteredCommands = computed(() => {
  return commands.value.filter((command) => {
    return command.label.toLowerCase().includes(commandQuery.value.toLowerCase());
  });
});

function openCommandSelection() {
  commanding.value = true;
  nextTick(() => ((commandInputRefFocused.value = true), commandButtonRef.value?.$el.click()));
}

function stopCommanding() {
  commanding.value = false;
  query.value = "";
  nextTick(() => spanRef.value?.focus());
}

function selectCommand(command: Command) {
  commanding.value = false;
  query.value = "";
  command.action();
}

function focus() {
  spanRef.value?.focus();
  commanding.value = false;
}

function blur() {
  spanRef.value?.blur();
  commandInputRefFocused.value = false;
  commanding.value = false;
}

const appearance = useAppearance();

defineExpose({
  focus: (position: "first" | "last" = "first") => focus(),
  blur,
  loading: ref(false),
});
</script>
<template>
  <span class="flex w-full flex-row items-center outline-none" @click="spanRef?.focus()">
    <EditableSpan
      ref="spanRef"
      v-if="!commanding"
      v-model="query"
      :readonly="context.readonly.value"
      @navigate-up="context.navigateUp"
      @navigate-down="context.navigateDown"
      @navigate-left="emit('navigateLeft')"
      @navigate-right="emit('navigateRight')"
      @enter="context.insertBelow"
      @escape="emit('escape')"
      @delete-left="context.deleteSelfLeft"
    />
    <!-- Command selection -->
    <Combobox
      v-else
      as="div"
      class="relative flex w-full flex-col"
      @update:model-value="selectCommand($event)"
      by="label"
    >
      <!-- Hidden button to manage focus programmatically -->
      <ComboboxButton class="hidden" ref="commandButtonRef" />
      <span class="flex flex-row items-baseline">
        /
        <ComboboxInput
          as="input"
          ref="commandInputRef"
          @change="commandQuery = $event.target.value"
          spellcheck="false"
          class="w-full min-w-0 border-0 bg-transparent p-0 outline-none ring-0 focus:ring-0"
          :class="[appearance.textSmall ? 'text-sm' : 'text-md']"
          @keydown.backspace.exact="commandQuery.length > 0 || stopCommanding()"
          @keydown.escape.prevent="emit('escape')"
        />
      </span>
      <FadeTransition>
        <ComboboxOptions
          ref="commandOptionsRef"
          class="absolute top-7 z-20 flex max-h-80 w-[340px] flex-col gap-1 overflow-y-auto rounded-sm bg-white p-1 py-1 shadow-md ring-1 ring-orange-900 ring-opacity-20 focus:outline-none"
        >
          <div v-if="filteredCommands.length == 0" class="w-full px-2 py-1">
            <span class="text-gray-700">No results</span>
          </div>
          <ComboboxOption
            v-for="(command, i) in filteredCommands"
            :key="command.label"
            :value="command"
            v-slot="{ active }"
          >
            <div
              v-if="i == 0 || command.group != filteredCommands[i - 1]?.group"
              class="select-none px-2 py-1 text-xs font-semibold tracking-wide text-gray-500"
            >
              {{ command.group?.name }}
            </div>
            <li
              class="flex flex-row items-center justify-between gap-3"
              :class="[
                'cursor-pointer select-none px-2 py-0.5',
                active ? 'bg-orange-100 text-gray-900' : 'text-gray-900',
              ]"
            >
              <div class="py-1">
                <div class="relative h-8 w-8 rounded-md bg-orange-500">
                  <component :is="command.icon" class="absolute left-1.5 top-1.5 h-5 w-5 text-white" />
                </div>
              </div>
              <div class="flex flex-1 flex-col">
                <span class="font-semibold text-orange-600">
                  {{ command.label }}
                </span>
                <span class="text-xs text-gray-700"> {{ command.description }}. </span>
              </div>
            </li>
          </ComboboxOption>
        </ComboboxOptions>
      </FadeTransition>
    </Combobox>
    <!-- Empty dots / prompt -->
    <div
      v-if="
        showDots && context.statement.value.type == StatementType.Blank && query?.length == 0 && context.focused.value
      "
      class="h-full w-full select-none items-center group-hover:opacity-100"
    >
      <span class="text-gray-400" v-if="!context.editing.value"><EllipsisHorizontalIcon class="h-4 w-4" /></span>
      <span class="text-gray-400" v-else>Press '/' for commands, type for text...</span>
    </div>
  </span>
</template>
