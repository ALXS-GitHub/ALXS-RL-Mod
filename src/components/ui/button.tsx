import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import { LoaderCircle } from "lucide-react";
import type { ButtonHTMLAttributes } from "react";
import { cn } from "@/lib/cn";

const buttonVariants = cva(
  "inline-flex select-none items-center justify-center gap-1.5 whitespace-nowrap font-medium outline-none transition-colors duration-150 disabled:pointer-events-none disabled:opacity-40 [&_svg]:size-4 [&_svg]:shrink-0",
  {
    variants: {
      variant: {
        primary:
          "bg-[var(--color-accent)] text-white shadow-[inset_0_1px_0_rgb(255_255_255/0.14)] hover:bg-[color-mix(in_oklab,var(--color-accent)_86%,white)]",
        secondary:
          "border border-line bg-white/[0.06] text-fg hover:border-line-strong hover:bg-white/[0.09]",
        ghost: "text-fg-muted hover:bg-white/[0.06] hover:text-fg",
        outline: "border border-line-strong text-fg hover:bg-white/[0.05]",
        danger: "bg-danger text-white hover:bg-[color-mix(in_oklab,var(--color-danger)_86%,white)]",
        success: "bg-success text-[#04140d] hover:bg-[color-mix(in_oklab,var(--color-success)_86%,white)]",
      },
      size: {
        sm: "h-7 rounded-[6px] px-2.5 text-xs [&_svg]:size-3.5",
        md: "h-8 rounded-[7px] px-3 text-[13px]",
        lg: "h-9 rounded-[8px] px-4 text-[13px]",
        icon: "size-8 rounded-[7px]",
        "icon-sm": "size-7 rounded-[6px] [&_svg]:size-3.5",
      },
    },
    defaultVariants: { variant: "secondary", size: "md" },
  },
);

export interface ButtonProps
  extends ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  /** Render the child element instead of a `<button>` (e.g. a router link). */
  asChild?: boolean;
  loading?: boolean;
}

export function Button({
  className,
  variant,
  size,
  asChild = false,
  loading = false,
  disabled,
  children,
  ...props
}: ButtonProps) {
  const Comp = asChild ? Slot : "button";
  return (
    <Comp
      className={cn(buttonVariants({ variant, size }), className)}
      disabled={disabled || loading}
      aria-busy={loading || undefined}
      {...props}
    >
      {/* Slot (asChild) needs exactly one child: no spinner in that mode. */}
      {asChild ? (
        children
      ) : (
        <>
          {loading ? <LoaderCircle className="animate-spin" aria-hidden /> : null}
          {children}
        </>
      )}
    </Comp>
  );
}

export { buttonVariants };
