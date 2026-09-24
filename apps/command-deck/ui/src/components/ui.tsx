// shadcn-style primitives: one level of surface, hairline borders,
// Lucide icons upstream. No nested cards, no accent border-lefts.
import { clsx, type ClassValue } from "clsx";
import type { ReactNode } from "react";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}

export function Button({
  children,
  onClick,
  disabled,
  variant = "default",
  title,
}: {
  children: ReactNode;
  onClick?: () => void;
  disabled?: boolean;
  variant?: "default" | "ghost" | "danger";
  title?: string;
}) {
  return (
    <button
      type="button"
      title={title}
      onClick={onClick}
      disabled={disabled}
      className={cn(
        "inline-flex min-h-[36px] cursor-pointer items-center gap-2 rounded-lg px-3 text-sm font-medium transition-colors duration-150",
        "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent",
        "disabled:cursor-not-allowed disabled:opacity-50",
        variant === "default" && "bg-accent text-accent-ink hover:brightness-110 active:brightness-95",
        variant === "ghost" && "border border-edge bg-transparent text-dim hover:border-faint hover:text-ink",
        variant === "danger" && "bg-bad/15 text-bad hover:bg-bad/25",
      )}
    >
      {children}
    </button>
  );
}

export function Card({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div className={cn("rounded-xl border border-edge bg-panel", className)}>{children}</div>
  );
}

const BADGE_TONES: Record<string, string> = {
  sky: "bg-sky-400/10 text-sky-300",
  amber: "bg-amber-400/10 text-amber-300",
  blue: "bg-accent/10 text-accent",
  emerald: "bg-emerald-400/10 text-emerald-300",
  zinc: "bg-zinc-400/10 text-zinc-300",
  rose: "bg-rose-400/10 text-rose-300",
};

export function Badge({ tone = "zinc", children }: { tone?: string; children: ReactNode }) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium",
        BADGE_TONES[tone] ?? BADGE_TONES.zinc,
      )}
    >
      {children}
    </span>
  );
}

export function Skeleton({ className }: { className?: string }) {
  return <div aria-hidden className={cn("skeleton rounded-lg", className)} />;
}

export function EmptyState({
  icon,
  title,
  hint,
}: {
  icon: ReactNode;
  title: string;
  hint: string;
}) {
  return (
    <div className="flex flex-col items-center gap-2 rounded-xl border border-dashed border-edge px-6 py-10 text-center">
      <div className="text-faint">{icon}</div>
      <p className="font-display text-lg font-semibold">{title}</p>
      <p className="max-w-[52ch] text-sm text-dim">{hint}</p>
    </div>
  );
}

export function Field({
  label,
  children,
  hint,
}: {
  label: string;
  children: ReactNode;
  hint?: string;
}) {
  return (
    <label className="block">
      <span className="mb-1 block text-sm font-medium text-dim">{label}</span>
      {children}
      {hint && <span className="mt-1 block text-xs text-faint">{hint}</span>}
    </label>
  );
}

export function TextInput(props: React.InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      {...props}
      className={cn(
        "h-10 w-full rounded-lg border border-edge bg-abyss px-3 font-mono text-sm text-ink",
        "placeholder:text-faint focus:border-accent focus:outline-none",
        props.className,
      )}
    />
  );
}
