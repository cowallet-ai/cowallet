import { type ButtonHTMLAttributes } from "react";

type Variant = "primary" | "accent" | "outline" | "ghost";
type Size = "default" | "lg";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
  pill?: boolean;
  full?: boolean;
}

const variantStyles: Record<Variant, string> = {
  primary: "bg-ink-1 text-paper border-transparent",
  accent: "bg-accent text-white border-transparent shadow-[0_4px_14px_-2px_rgba(217,119,87,0.35)]",
  outline: "bg-transparent border-line-strong text-ink-1",
  ghost: "bg-transparent border-transparent text-ink-2",
};

const sizeStyles: Record<Size, string> = {
  default: "px-[18px] py-3 text-[14.5px] rounded-[14px]",
  lg: "px-5 py-[15px] text-[15px] rounded-[999px]",
};

export function Button({
  variant = "primary",
  size = "default",
  pill = false,
  full = false,
  className = "",
  children,
  ...props
}: ButtonProps) {
  return (
    <button
      className={`
        inline-flex items-center justify-center gap-1.5 font-medium border cursor-pointer
        transition-transform duration-75 active:scale-[0.98]
        ${variantStyles[variant]}
        ${sizeStyles[size]}
        ${pill ? "!rounded-[999px]" : ""}
        ${full ? "w-full" : ""}
        ${className}
      `}
      {...props}
    >
      {children}
    </button>
  );
}
