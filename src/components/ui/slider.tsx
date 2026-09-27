import * as SliderPrimitive from "@radix-ui/react-slider";
import type { ComponentProps } from "react";
import { cn } from "@/lib/cn";

export function Slider({ className, ...props }: ComponentProps<typeof SliderPrimitive.Root>) {
  return (
    <SliderPrimitive.Root
      className={cn("relative flex h-5 w-full touch-none select-none items-center", className)}
      {...props}
    >
      <SliderPrimitive.Track className="relative h-1 grow overflow-hidden rounded-full bg-white/[0.12]">
        <SliderPrimitive.Range className="absolute h-full bg-[var(--color-accent)]" />
      </SliderPrimitive.Track>
      {(props.value ?? props.defaultValue ?? [0]).map((_, i) => (
        <SliderPrimitive.Thumb
          // biome-ignore lint/suspicious/noArrayIndexKey: thumbs are positional
          key={i}
          className="block size-3.5 rounded-full bg-white shadow-[0_1px_3px_rgb(0_0_0/0.5)] outline-none"
        />
      ))}
    </SliderPrimitive.Root>
  );
}
