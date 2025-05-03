<script lang="ts" setup>
import { makeType } from "@/language/core/type";
import {
  BenchType,
  Continent,
  NodeType,
  TypeKind,
  UserWizardViewStage,
  ViewData,
  type NodeReferenceData,
} from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { benchPtr } from "@/system/client";
import { canvas, goToBench } from "@/system/space";
import { logIn, signUp, user } from "@/system/user";
import { fireCommandById } from "@/ui/command";
import { makeIcon } from "@/ui/icon";
import { getViewComponentChildren, isVueInstanceOf } from "@/ui/view";
import { DEFAULT_REGION_BY_CONTINENT } from "@/utils/region";
import { type FocusAnchor, type ViewEmits, type ViewExpose } from "@/views/common";
import NativeInput from "@/views/content/NativeInput.vue";
import Picker from "@/views/content/Picker.vue";
import Button from "@/views/controls/Button.vue";
import ThreeIcon from "@/views/helpers/ThreeIcon.vue";
import { ref, toRef, type Ref } from "vue";

const props = defineProps<{ self?: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Pick<ViewData, "title">>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");

const stage = ref<UserWizardViewStage>(UserWizardViewStage.LOG_IN);
const name: Ref<string> = ref("");
const slug: Ref<string> = ref("");
const email: Ref<string> = ref("");
const password: Ref<string> = ref("");
const continent: Ref<Continent> = ref(Continent.EUROPE);
// NOTE: auto-set geolocation once we have more than once region
// watchEffect(() => {
//   if (GEOLOCATION.value?.continent != null) {
//     const defaultRegion = DEFAULT_REGION_BY_AREA[GEOLOCATION.value.continent];
//     if (defaultRegion != null) {
//       continent.value = defaultRegion;
//     }
//   }
// });
const isActive = ref(false);
const lastError: Ref<string | null> = ref(null);
const isCreating = ref(false);

function clear() {
  name.value = "";
  slug.value = "";
  email.value = "";
  password.value = "";
}

function switchStage() {
  if (stage.value == UserWizardViewStage.LOG_IN) {
    stage.value = UserWizardViewStage.SIGN_UP;
  } else if (stage.value == UserWizardViewStage.SIGN_UP) {
    stage.value = UserWizardViewStage.LOG_IN;
  } else {
    throw new Error(`unexpected registration stage: ${stage.value}`);
  }
}

async function submit() {
  isActive.value = true;
  lastError.value = null;
  try {
    if (stage.value == UserWizardViewStage.SIGN_UP) {
      isCreating.value = true;
      try {
        const region = DEFAULT_REGION_BY_CONTINENT[continent.value];
        if (region == null) {
          throw new Error(`unexpected continent: ${continent.value}`);
        }
        const { user } = await signUp(
          { name: name.value, slug: slug.value, email: email.value, region: region },
          password.value,
        );
        await goToBench({ bench: user.benchPtr as TypedNodeReferenceData<NodeType.BENCH> });
        await fireCommandById("space.create.page");
      } finally {
        isCreating.value = false;
      }
    } else if (stage.value == UserWizardViewStage.LOG_IN) {
      const { user } = await logIn({ slug: slug.value }, password.value);
      // if we're outside a Bench and have a Bench, go home
      if (user.benchPtr != null && benchPtr.value == null) {
        await goToBench({ bench: user.benchPtr as TypedNodeReferenceData<NodeType.BENCH> });
      }
    } else {
      throw new Error(`unexpected registration stage: ${stage.value}`);
    }
    clear();
  } catch (e) {
    lastError.value = (e as Error).message ?? "Unknown error";
  } finally {
    isActive.value = false;
  }
}

const instance = canvas.registerView(self, id);
function focus(anchor?: FocusAnchor | NodeReferenceData) {
  const childViews = getViewComponentChildren(instance);
  if (anchor != "bottom") {
    return childViews.find((v) => isVueInstanceOf(v, NativeInput));
  } else {
    return childViews.reverse().find((v) => isVueInstanceOf(v, Button));
  }
}

defineExpose<ViewExpose>({ id, self, focus });
</script>
<template>
  <div class="flex h-full flex-row divide-x divide-gray-200">
    <!-- TODO :UX!: make UserWizard not suck -->
    <!-- Image -->
    <div class="flex h-full w-[55%] shrink-0 flex-col items-center justify-center bg-yellow-400">
      <ThreeIcon class="h-[20%] w-[20%]" />
    </div>

    <!-- Form -->
    <!-- Added overflow-hidden to prevent content from exceeding the flex-1 width -->
    <div
      class="mx-auto flex h-full max-w-lg flex-1 flex-col justify-center overflow-hidden rounded-sm px-9 text-left text-gray-900"
    >
      <!-- Header -->
      <div>
        <h2 class="text-4xl font-semibold">Bench</h2>
        <p class="mt-2 text-lg">
          <span v-if="stage == UserWizardViewStage.LOG_IN">Log into an existing Bench account.</span>
          <span v-else>Create a new Bench account.</span>
        </p>
      </div>
      <!-- Data -->
      <!-- Removed max-w-full, relying on parent flex-1 constraint -->
      <div class="mt-5 space-y-2">
        <!-- Using space-y for vertical spacing instead of flex gap -->
        <div v-if="stage == UserWizardViewStage.SIGN_UP">
          <span class="block pb-0.5 font-medium">Name</span>
          <NativeInput
            id="name"
            ref="nameRef"
            v-model="name"
            :icon="makeIcon({ faName: 'fas fa-user' })"
            name="Name"
            title="Name"
            is-input
          />
        </div>
        <div>
          <span class="block pb-0.5 font-medium">Username</span>
          <NativeInput
            id="slug"
            ref="slugRef"
            v-model="slug"
            :icon="makeIcon({ faName: 'fas fa-hashtag' })"
            name="slug"
            title="Username"
            is-input
          />
        </div>
        <div v-if="stage == UserWizardViewStage.SIGN_UP">
          <span class="block pb-0.5 font-medium">Email</span>
          <NativeInput
            id="email"
            v-model="email"
            :icon="makeIcon({ faName: 'fas fa-at' })"
            name="Email"
            title="Email"
            is-input
          />
        </div>
        <!-- NOTE :UX: add passowrd feedback (see https://zxcvbn-ts.github.io/zxcvbn/)? -->
        <div>
          <span class="block pb-0.5 font-medium">Password</span>
          <NativeInput
            id="password"
            v-model="password"
            :icon="makeIcon({ faName: 'fas fa-key' })"
            name="Password"
            title="Password"
            is-input
            :value-type="makeType({ isSecret: true })"
          />
        </div>
        <div v-if="stage == UserWizardViewStage.SIGN_UP">
          <span class="block pb-0.5 font-medium">Region</span>
          <Picker
            id="region"
            v-model="continent"
            :icon="makeIcon({ faName: 'fas fa-globe' })"
            name="Region"
            title="Region"
            is-input
            :value-type="
              makeType({ kind: TypeKind.ENUM, benchType: BenchType.CONTINENT, isList: false, isRequired: true })
            "
          />
        </div>
      </div>
      <!-- Commands -->
      <div class="mt-7">
        <Button
          v-if="!user"
          id="submit"
          name="Submit"
          :icon="makeIcon({ faName: 'fas fa-arrow-right-from-bracket' })"
          :title="stage === UserWizardViewStage.LOG_IN ? 'Log in' : 'Sign up'"
          class="w-full"
          :is-disabled="isActive"
          :is-loading="isActive"
          @click="submit"
        />
        <Button
          v-if="!user"
          id="switch"
          name="Switch"
          :icon="makeIcon({ faName: 'fas fa-shuffle' })"
          :title="stage === UserWizardViewStage.LOG_IN ? 'Sign up' : 'Log in'"
          class="mt-2 w-full"
          @click="() => switchStage()"
        />
      </div>
      <!-- Error -->
      <!-- Simplified error layout slightly -->
      <div v-if="lastError" class="mt-3 space-y-1 border-t border-t-gray-200 pt-3">
        <div class="text-danger-600 flex flex-row items-center gap-x-2">
          <i class="fas fa-circle-exclamation" />
          <span class="font-medium">Error</span>
        </div>
        <div>
          <span class="text-gray-500">{{ lastError }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
