import * as TabsPrimitive from "@radix-ui/react-tabs";
import { motion } from "motion/react";
import { type ComponentProps, createContext, useContext, useId } from "react";
import { cn } from "@/lib/cn";

const TabsCtx = createContext<{ value?: string; layoutId: string }>({ layoutId: "tabs" });

/** Controlled tabs; a flat pill slides under the active trigger. */
export function Tabs({ value, className, ...props }: ComponentProps<typeof TabsPrimitive.Root>) {
  const layoutId = useId();
  return (
    <TabsCtx.Provider value={{ value, layoutId }}>
      <TabsPrimitive.Root value={value} className={cn("flex flex-col gap-4", className)} {...props} />
    </TabsCtx.Provider>
  );
}

export function TabsList({ className, ...props }: ComponentProps<typeof TabsPrimitive.List>) {
  return (
    <TabsPrimitive.List
      className={cn(
        "inline-flex w-fit items-center gap-0.5 rounded-[8px] border border-line bg-white/[0.03] p-0.5",
        className,
      )}
      {...props}
    />
  );
}

export function TabsTrigger({
  className,
  value,
  children,
  ...props
}: ComponentProps<typeof TabsPrimitive.Trigger>) {
  const ctx = useContext(TabsCtx);
  const active = ctx.value === value;
  return (
    <TabsPrimitive.Trigger
      value={value}
      className={cn(
        "relative inline-flex h-7 items-center gap-1.5 rounded-[6px] px-3 text-[13px] font-medium text-fg-muted transition-colors hover:text-fg data-[state=active]:text-fg [&_svg]:size-3.5",
        className,
      )}
      {...props}
    >
      {active ? (
        <motion.span
          layoutId={ctx.layoutId}
          className="absolute inset-0 -z-10 rounded-[6px] bg-white/[0.09]"
          transition={{ type: "spring", stiffness: 500, damping: 40 }}
        />
      ) : null}
      {children}
    </TabsPrimitive.Trigger>
  );
}

export function TabsContent({ className, ...props }: ComponentProps<typeof TabsPrimitive.Content>) {
  return <TabsPrimitive.Content className={cn("outline-none", className)} {...props} />;
}
