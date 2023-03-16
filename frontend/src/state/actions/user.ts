import { provideGlobalAction } from "@/state/actions";
import { useAuth } from "@/state/auth";
import { useUserOps } from "@/state/operations/user";
import { computed } from "vue";
import { useRouter } from "vue-router";

export function useUserActions() {
  const ops = useUserOps();
  const router = useRouter();
  const auth = useAuth();

  const signup = provideGlobalAction({
    id: "user.startSignup",
    label: "Signup",
    shortcuts: [],
    enabled: computed(() => !auth.loggedIn),
    apply: () => {
      router.push({ name: "Signup" });
    },
  });

  const login = provideGlobalAction({
    id: "user.startLogin",
    label: "Login",
    shortcuts: [],
    enabled: computed(() => !auth.loggedIn),
    apply: () => {
      router.push({ name: "Login" });
    },
  });

  const logout = provideGlobalAction({
    id: "user.logout",
    label: "Logout",
    shortcuts: [],
    apply: () => ops.logout(),
  });

  return { signup, login, logout };
}
