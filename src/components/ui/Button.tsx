import { JSX, splitProps } from "solid-js";
import styles from "./Button.module.css";

interface ButtonProps extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "ghost" | "danger";
  size?: "sm" | "md";
}

export function Button(props: ButtonProps) {
  const [local, rest] = splitProps(props, ["variant", "size", "children", "class"]);

  const cls = () =>
    [
      styles.btn,
      styles[local.variant ?? "primary"],
      styles[local.size ?? "md"],
      local.class ?? "",
    ].join(" ");

  return (
    <button class={cls()} {...rest}>
      {local.children}
    </button>
  );
}
