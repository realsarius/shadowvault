import { JSX } from "solid-js";
import styles from "./Badge.module.css";

type BadgeVariant = "success" | "error" | "warning" | "running" | "neutral";

export function Badge(props: { variant: BadgeVariant; children: JSX.Element }) {
  return (
    <span class={`${styles.badge} ${styles[props.variant]}`}>
      {props.variant === "running" && <span class={`${styles.dot} pulse`} />}
      {props.children}
    </span>
  );
}
