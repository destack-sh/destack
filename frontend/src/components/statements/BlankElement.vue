<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import EditableSpan from "@/components/basic/EditableSpan.vue";
import { useAppearance } from "@/state/appearance";
import { Combobox, ComboboxOption, ComboboxInput, ComboboxOptions, ComboboxButton } from "@headlessui/vue";
import { useFocus, type MaybeElementRef } from "@vueuse/core";
import { computed, nextTick, ref, watch, type Ref, toRef } from "vue";
import { StatementType } from "@/gql/graphql";
import { EllipsisHorizontalIcon } from "@heroicons/vue/24/outline";
import { useActiveScroll } from "@/composables/useScroll";
import { usePanelContext } from "@/state/bench";
import type { StatementProps } from "@/components/statements";
import type { StatementEmit } from "@/components/statements";
import { closeTransaction, openTransaction, useOperations } from "@/state/operations";
import { useStatementMorph, type MorphCommand } from "@/state/statement";

const props = defineProps<Pick<StatementProps, "statement" | "readonly" | "bounding" | "focused" | "editing">>();
const emit = defineEmits<StatementEmit>();

const appearance = useAppearance();
const isInTopHalfOfPanel = computed(() => props.bounding.y.value < panel.size.value.height / 2);

const query: Ref<string> = ref("");
const inputRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
const panel = usePanelContext();
const ops = useOperations();

// open/close inserting and auto-convert to text on anything else
watch(query, (q) => {
  if (q == "") {
    inserting.value = false;
  } else if (q == " ") {
    inputRef.value?.clear();
    emit("launchAssist", "");
  } else if (q == "/") {
    openInputSelection();
  } else if (q.startsWith("#")) {
    const headingLevel = (q.match(/^#+ /)?.[0].length ?? 0) - 1;
    if (headingLevel > 0 && headingLevel < 4) {
      ops.statement.morph(null, props.statement.id, props.statement, {
        type: StatementType.Text,
        headingLevel: headingLevel,
      });
    }
  } else if (!inserting.value) {
    const tx = openTransaction();
    ops.statement.morph(tx, props.statement.id, props.statement, { type: StatementType.Text });
    ops.symbol.updateStatementText(tx, props.statement.id, "", q);
    closeTransaction(tx);
  }
});

// input selection dropdown
const inserting: Ref<boolean> = ref(false);
const inputQuery: Ref<string> = ref("");
const inputInputRef: Ref<InstanceType<typeof ComboboxInput> | null> = ref(null);
const inputButtonRef: Ref<InstanceType<typeof ComboboxButton> | null> = ref(null);
const inputOptionsRef: Ref<InstanceType<typeof ComboboxOptions> | null> = ref(null);
const { focused: inputInputRefFocused } = useFocus(inputInputRef as MaybeElementRef);

useActiveScroll(computed(() => inputOptionsRef.value?.$el));

function morphToText() {
  query.value = "";
  nextTick(() => inputRef.value?.focus());
}

function openInputSelection() {
  inserting.value = true;
  nextTick(() => ((inputInputRefFocused.value = true), inputButtonRef.value?.$el.click()));
}

function stopInserting() {
  inserting.value = false;
  query.value = "";
  nextTick(() => inputRef.value?.focus());
}

function selectInput(input: MorphCommand) {
  inserting.value = false;
  query.value = "";
  doMorph(props.statement, { ...input.identity, name: props.statement.name }, input);
  input.action?.();
}

const { filteredCommands, doMorph } = useStatementMorph(toRef(props, "statement"), inserting, {
  query: inputQuery,
  includeTemplates: true,
});

defineExpose({
  focus: (position: "first" | "last" = "first") => {
    inputRef.value?.focus();
    inserting.value = false;
  },
  blur: () => {
    inputRef.value?.blur();
    inputInputRefFocused.value = false;
    inserting.value = false;
  },
  loading: ref(false),
});
</script>
<template>
  <div class="flex w-full flex-row items-center outline-none" @click="inputRef?.focus()">
    <EditableSpan
      ref="inputRef"
      v-if="!inserting"
      v-model="query"
      :readonly="readonly"
      @navigate-up="emit('navigateUp')"
      @navigate-down="emit('navigateDown')"
      @navigate-left="emit('navigateLeft')"
      @navigate-right="emit('navigateRight')"
      @enter-left="emit('enterLeft')"
      @enter="emit('enter')"
      @enter-right="emit('enterRight')"
      @delete-left="emit('deleteLeft')"
      @paste.prevent="emit('paste')"
    />
    <!-- Empty dots / prompt -->
    <div
      v-if="focused && !inserting && query == ''"
      class="h-full w-full select-none items-center group-hover:opacity-100"
    >
      <span class="text-gray-400" v-if="!editing"><EllipsisHorizontalIcon class="h-4 w-4" /></span>
      <span class="text-gray-400" v-else
        >Press
        <button
          class="rounded-sm px-1 font-semibold ring-1 ring-inset ring-gray-300 hover:bg-gray-100"
          @click.stop="emit('launchAssist', '')"
        >
          space
        </button>
        for assist,
        <button
          class="rounded-sm px-1 font-semibold ring-1 ring-inset ring-gray-300 hover:bg-gray-100"
          @click.stop="openInputSelection()"
        >
          /
        </button>
        to insert...
      </span>
    </div>
    <!-- Input selection -->
    <Combobox
      v-if="inserting"
      as="div"
      class="relative flex w-full flex-col"
      @update:model-value="selectInput($event)"
      by="label"
    >
      <!-- Hidden button to manage focus programmatically -->
      <ComboboxButton class="hidden" ref="inputButtonRef" />
      <span class="flex flex-row items-baseline">
        /
        <ComboboxInput
          as="input"
          ref="inputInputRef"
          @change="inputQuery = $event.target.value"
          spellcheck="false"
          class="w-full min-w-0 border-0 bg-transparent p-0 outline-none ring-0 focus:ring-0"
          :class="[appearance.textSmall ? 'text-sm' : 'text-md']"
          @keydown.backspace.exact="inputQuery.length > 0 || stopInserting()"
          @keydown.escape.prevent="morphToText(), emit('escape')"
        />
      </span>
      <!-- Prevent scroll and capture click outside -->
      <div
        v-if="inserting"
        class="fixed left-0 top-0 z-40 h-full w-full overscroll-none"
        @click.stop="inserting = false"
      />
      <!-- Morph input popup options -->
      <FadeTransition>
        <ComboboxOptions
          ref="inputOptionsRef"
          class="absolute z-50 flex h-fit max-h-[360px] w-[340px] flex-col gap-1 overflow-y-auto overflow-x-hidden rounded-sm bg-white p-1 py-1 shadow-md ring-1 ring-orange-900 ring-opacity-20 focus:outline-none"
          :class="[isInTopHalfOfPanel ? 'top-7' : 'bottom-7']"
        >
          <div v-if="filteredCommands.length == 0" class="w-full px-2 py-1">
            <span class="text-gray-700">No results</span>
          </div>
          <ComboboxOption
            v-for="(input, i) in filteredCommands"
            :key="i + input.label"
            :value="input"
            v-slot="{ active }"
          >
            <div
              v-if="i == 0 || input.group?.name != filteredCommands[i - 1]?.group?.name"
              class="select-none px-2 py-1 text-xs font-semibold tracking-wide text-gray-500"
            >
              {{ input.group?.name }}
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
                  <component :is="input.iconSolid" class="absolute left-1.5 top-1.5 h-5 w-5 text-white" />
                </div>
              </div>
              <div class="flex flex-1 flex-col">
                <span class="font-semibold text-orange-600">
                  {{ input.label }}
                </span>
                <span class="max-w-full truncate whitespace-nowrap text-xs text-gray-700">
                  {{ input.description }}
                </span>
              </div>
            </li>
          </ComboboxOption>
        </ComboboxOptions>
      </FadeTransition>
    </Combobox>
  </div>
</template>
