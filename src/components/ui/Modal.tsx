import { JSX, Show } from "solid-js";
import styles from "./Modal.module.css";

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title: string;
  children: JSX.Element;
  footer?: JSX.Element;
}

export function Modal(props: ModalProps) {
  return (
    <Show when={props.open}>
      <div
        class={styles.overlay}
        onClick={(e) => { if (e.target === e.currentTarget) props.onClose(); }}
      >
        <div class={styles.dialog}>
          <div class={styles.header}>
            <span class={styles.title}>{props.title}</span>
            <button class={styles.closeBtn} onClick={props.onClose}>✕</button>
          </div>
          <div class={styles.body}>{props.children}</div>
          <Show when={props.footer}>
            <div class={styles.footer}>{props.footer}</div>
          </Show>
        </div>
      </div>
    </Show>
  );
}
