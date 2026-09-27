import { Search, X } from "lucide-react";
import type { InputHTMLAttributes } from "react";
import { cn } from "@/lib/cn";

export function Input({ className, ...props }: InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      className={cn(
        "h-8 w-full rounded-[7px] border border-line bg-black/25 px-2.5 text-[13px] text-fg placeholder:text-fg-subtle outline-none transition-colors hover:border-line-strong focus:border-[var(--color-accent)]",
        className,
      )}
      {...props}
    />
  );
}

interface SearchInputProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "onChange"> {
  value: string;
  onValueChange: (value: string) => void;
}

export function SearchInput({ value, onValueChange, className, ...props }: SearchInputProps) {
  return (
    <div className={cn("relative", className)}>
      <Search className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-fg-subtle" />
      <Input
        value={value}
        onChange={(e) => onValueChange(e.target.value)}
        className="pr-8 pl-8"
        type="search"
        {...props}
      />
      {value ? (
        <button
          type="button"
          aria-label="Clear"
          onClick={() => onValueChange("")}
          className="absolute top-1/2 right-1.5 grid size-5 -translate-y-1/2 place-items-center rounded-[4px] text-fg-subtle hover:bg-white/10 hover:text-fg"
        >
          <X className="size-3.5" />
        </button>
      ) : null}
    </div>
  );
}
