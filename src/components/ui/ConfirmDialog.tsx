import { Show } from "solid-js";
import { TbOutlineAlertTriangle } from "solid-icons/tb";
import { Button } from "./Button";
import { t } from "../../i18n";
import styles from "./ConfirmDialog.module.css";

interface Props {
  open: boolean;
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmDialog(props: Props) {
  return (
    <Show when={props.open}>
      <div class={styles.overlay} onClick={(e) => { if (e.target === e.currentTarget) props.onCancel(); }}>
        <div class={styles.dialog}>
          <div class={styles.icon}><TbOutlineAlertTriangle size={24} /></div>
          <p class={styles.message}>{props.message}</p>
          <div class={styles.actions}>
            <Button variant="ghost" size="sm" onClick={props.onCancel}>{t("btn_cancel")}</Button>
            <Button variant="danger" size="sm" onClick={props.onConfirm}>{t("btn_delete")}</Button>
          </div>
        </div>
      </div>
    </Show>
  );
}
