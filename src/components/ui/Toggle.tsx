import { Show } from "solid-js";
import styles from "./Toggle.module.css";

export function Toggle(props: { value: boolean; onChange: (v: boolean) => void; label?: string }) {
  return (
    <label class={styles.label}>
      <div
        class={styles.track}
        data-active={String(props.value)}
        onClick={() => props.onChange(!props.value)}
      >
        <div class={styles.thumb} />
      </div>
      <Show when={props.label}>
        <span class={styles.text}>{props.label}</span>
      </Show>
    </label>
  );
}
