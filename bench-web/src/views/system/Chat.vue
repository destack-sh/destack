<script lang="ts" setup>
import { ViewData, NodeType, BoxData, Variant, ObjectType, Orientation, TextData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { canvas, pkg } from "@/system/space";
import { computed, ref, toRef, type Ref } from "vue";
import { useExistingConnection } from "@/system/connection";
import Scroll from "@/views/containers/Scroll.vue";
import { ScrollbarWidth } from "@/utils/layout";
import { useElementSize } from "@vueuse/core";
import Text from "@/views/content/Text.vue";
import { tsToDt, formatDurationFromNow } from "@/utils/time";
import { ICON_BY_NODE_TYPE, IconInline, getNodeIcon } from "@/system/icon";
import { isTextEmpty } from "@/system/text";

const HEADER_HEIGHT = 40;
const MAX_WIDTH = 800;
const MIN_INPUT_HEIGHT = 40;

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; size: Required<Pick<BoxData, "width" | "height">> } & Partial<
    Pick<ViewData, "title" | "nodePtr" | "variant">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const maxInputHeight = computed(() => (props.size.height - HEADER_HEIGHT) / 2);
const inputRef: Ref<HTMLDivElement | null> = ref(null);
const inputSize = useElementSize(inputRef);
const textRef: Ref<InstanceType<typeof Text> | null> = ref(null);

const threadPtr = toRef(props, "nodePtr") as Ref<TypedNodeReferenceData<NodeType.MESSAGE> | undefined>;
const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
const { graph: pkgGraph, connection: pkgConnection } = useExistingConnection(threadPtr);
const thread = pkgGraph.getRef(threadPtr);
const messages = pkgGraph.getChildrenRef(thread, NodeType.MESSAGE);
const ancestors = pkgGraph.getAncestorsRef(threadPtr, { includeSelf: false });
const context = computed(() => ancestors.value.find((n) => n.metatype != ObjectType.MESSAGE) ?? pkg.value);

const text: Ref<TextData | null> = ref(null);

const canSubmit = computed(() => !isTextEmpty(text.value));
function submit() {
  console.log("submit", text.value); // nocheckin
}

const isFocusedAbsolute = canvas.isFocusedAbsoluteRef(self);
canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, variants: [Variant.PRIMARY, Variant.COMPACT] });
</script>
<template>
  <div class="flex h-full w-full flex-col bg-white">
    <!-- Header -->
    <div class="group w-full border-b border-gray-200">
      <div
        class="mx-auto flex w-full max-w-full flex-row items-center pl-4 pr-5"
        :style="{ height: HEADER_HEIGHT + 'px', maxWidth: MAX_WIDTH + 'px' }"
      >
        <!-- Thread (local root message node) -->
        <div>
          <template v-if="thread">
            <i class="fas fa-message mr-1.5 w-5 text-center text-gray-700" />
            <span class="text-gray-900" :class="thread.title != null ? 'font-semibold' : ''">
              {{ thread.title ?? "Untitled thread" }}
            </span>
          </template>
          <template v-else>
            <i class="fas fa-message mr-1.5 w-5 text-center text-gray-400" />
            <span class="py-0.5 text-gray-400">New thread</span>
          </template>
        </div>
        <!-- Context (non-message parent node) -->
        <div class="ml-auto">
          <IconInline
            class="mr-1.5 w-5 text-center text-gray-400"
            v-bind="(context != null ? getNodeIcon(context) : null) ?? ICON_BY_NODE_TYPE[NodeType.BENCH]"
          />
          <span class="text-gray-400">
            {{ (context as any)?.name ?? "Package" }}
          </span>
          <!-- Select context node (move thread) -->
          <button class="ml-1.5 text-gray-400">
            <i class="fas fa-chevron-down" />
          </button>
        </div>
      </div>
    </div>

    <!-- Messages -->
    <Scroll
      v-if="thread"
      :size="{
        width: props.size.width,
        height: props.size.height - HEADER_HEIGHT - inputSize.height.value ?? MIN_INPUT_HEIGHT,
      }"
      :orientation="Orientation.VERTICAL"
      :track-width="ScrollbarWidth.md"
    >
      messages!
      <div v-for="message in messages" :key="message.id">
        {{ message?.id }}
      </div>
    </Scroll>
    <!-- Nothing here yet -->
    <div v-else-if="threadPtr">nocheckin: no messages</div>

    <!-- Create message -->
    <div ref="inputRef" class="group mt-auto border-t border-gray-200" @click="textRef?.focus">
      <div
        ref="inputRef"
        class="mx-auto flex flex-row items-end"
        :class="[variant != Variant.COMPACT ? 'gap-x-2.5 px-3 py-3' : 'gap-x-1.5 px-2.5 py-1.5']"
        :style="{ maxWidth: MAX_WIDTH + 'px', minHeight: MIN_INPUT_HEIGHT + 'px', maxHeight: maxInputHeight + 'px' }"
      >
        <!-- Upload -->
        <button
          class="h-fit self-end rounded border-gray-200 bg-white px-2 py-0.5 text-base hover:bg-gray-100"
          :class="
            isFocusedAbsolute ? 'text-gray-600 hover:text-primary-900' : 'text-gray-400 group-hover:text-gray-500'
          "
          @click.stop="() => {}"
        >
          <i class="fas fa-plus" />
        </button>
        <!-- Content -->
        <Text
          ref="textRef"
          class="my-0.5 w-full self-end hover:cursor-text"
          :variant="Variant.STEALTH"
          is-input
          :placeholder="`Message your Bench`"
					suppress-enter
          :model-value="text ?? undefined"
          @update:model-value="text = $event"
          @click.stop
          @keydown.enter.stop="submit"
        />
        <!-- Submit -->
        <button
          class="h-fit self-end rounded border-gray-200 bg-white px-2 py-0.5 text-base hover:bg-gray-100"
          :class="
            isFocusedAbsolute ? 'text-gray-600 hover:text-primary-900' : 'text-gray-400 group-hover:text-gray-500'
          "
          :disabled="!canSubmit"
          @click.stop="() => {}"
        >
          <i class="fas fa-arrow-up" />
        </button>
      </div>
    </div>
  </div>
</template>
