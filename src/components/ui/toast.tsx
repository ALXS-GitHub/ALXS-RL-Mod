import { Toaster as SonnerToaster, toast } from "sonner";
import { toAppError } from "@/lib/errors";
import i18n from "@/lib/i18n";

export function Toaster() {
  return (
    <SonnerToaster
      position="bottom-right"
      theme="dark"
      offset={20}
      toastOptions={{
        classNames: {
          toast: "glass-strong !rounded-lg !border-line-strong !text-fg !font-sans",
          description: "!text-fg-muted",
        },
      }}
    />
  );
}

/** Toast helpers with localised error messages. */
export const notify = {
  success: (message: string, description?: string) => toast.success(message, { description }),
  info: (message: string, description?: string) => toast(message, { description }),
  error: (error: unknown) => {
    const e = toAppError(error);
    toast.error(i18n.t(`errors.${e.kind}`, { defaultValue: i18n.t("errors.title") }), {
      description: e.message,
    });
  },
};
