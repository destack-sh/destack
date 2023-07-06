<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import EditableSpan from "@/components/basic/EditableSpan.vue";
import { TypeTag } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { STATEMENT_TYPE_BY_KEYWORD } from "@/state/type";
import { TypeFlag } from "@/state/module";
import { Combobox, ComboboxOption, ComboboxInput, ComboboxOptions, ComboboxButton } from "@headlessui/vue";
import { useFocus } from "@vueuse/core";
import { computed, nextTick, ref, watch, type Ref } from "vue";
import { StatementType } from "@/gql/graphql";
import { useStatementContext } from "@/state/statement";
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

// command/type selection dropdown
// (not sure if this is the best place to put it)
const commanding: Ref<boolean> = ref(false);
const commandQuery: Ref<string> = ref("");
const commandInputRef: Ref<InstanceType<typeof ComboboxInput> | null> = ref(null);
const commandButtonRef: Ref<InstanceType<typeof ComboboxButton> | null> = ref(null);
const commandOptionsRef: Ref<InstanceType<typeof ComboboxOptions> | null> = ref(null);
const { focused: commandInputRefFocused } = useFocus(commandInputRef);

useActiveScroll(computed(() => commandOptionsRef.value?.$el));

type Command = {
  label: string;
  description: string;
  action: () => void;
};

// TODO @UX: blank statement menu sucks
const commands = computed(() => {
  const commands: Command[] = [
    {
      label: "text",
      description: "Just start writing, documenting, whatever.",
      action: () => ((query.value = ""), nextTick(() => spanRef.value?.focus())),
    },
    {
      label: "task",
      description: "Instruct AI to do something.",
      action: () => (context.morpthToSymbol({ type: StatementType.Task }), emit("morphed")),
    },
    {
      label: "expectation",
      description: "Specify desired behaviour.",
      action: () => (context.morpthToSymbol({ type: StatementType.Expectation }), emit("morphed")),
    },
    {
      label: "type",
      description: "A structure type with multiple fields.",
      action: () => (
        context.morpthToSymbol({ type: StatementType.Type, rootTypeTag: TypeTag.Struct }), emit("morphed")
      ),
    },
    {
      label: "choice",
      description: "A choice type with multiple options.",
      action: () => (context.morpthToSymbol({ type: StatementType.Type, rootTypeTag: TypeTag.Enum }), emit("morphed")),
    },
    {
      label: "value",
      description: "A bit of configuration, secrets or flags.",
      action: () => (context.morpthToSymbol({ type: StatementType.Value, rootTypeFlags: 0 }), emit("morphed")),
    },
    {
      label: "dataset",
      description: "Data big and tiny, fast however you need it.",
      action: () => (
        context.morpthToSymbol({ type: StatementType.Dataset, rootTypeFlags: TypeFlag.IsArray }), emit("morphed")
      ),
    },
    {
      label: "code",
      description: "Custom logic in Python.",
      action: () => (context.morpthToSymbol({ type: StatementType.Code }), emit("morphed")),
    },
    {
      label: "reference",
      description: "Reuse another statement.",
      action: () => (context.morpthToSymbol({ type: StatementType.Reference }), emit("morphed")),
    },
    // { (soon)
    //   label: "block",
    //   description: "A group of related statements.",
    //   action: () => (context.morpthToSymbol({ type: StatementType.Block }), emit("morphed")),
    // },
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
  <span class="flex flex-row items-center outline-none">
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
      @delete-left="context.tryDeleteLeft"
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
          class="absolute top-7 z-20 flex max-h-64 w-80 flex-col gap-1 overflow-auto rounded-sm bg-white p-1 shadow-sm ring-1 ring-orange-900 ring-opacity-20 focus:outline-none"
        >
          <div v-if="filteredCommands.length == 0" class="w-full px-2 py-1">
            <span class="text-gray-700">No results</span>
          </div>
          <ComboboxOption v-for="command in filteredCommands" :key="command.label" :value="command" v-slot="{ active }">
            <li
              class="flex flex-col"
              :class="[
                'cursor-pointer select-none px-2 py-0.5',
                active ? 'bg-orange-100 text-gray-900' : 'text-gray-900',
              ]"
            >
              <span class="text-orange-600">
                {{ command.label }}
              </span>
              <span class="text-gray-700">
                {{ command.description }}
              </span>
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
