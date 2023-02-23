import { provideGlobalAction } from "@/state/actions";
import { useUserOps } from "@/state/operations/user";

export function useUserActions() {
  const ops = useUserOps();

  const logout = provideGlobalAction({
    id: "user.logout",
    label: "Logout",
    shortcuts: [],
    apply: () => ops.logout(),
  });

  return { logout };
}
