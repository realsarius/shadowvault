import styles from "./ErrorFallback.module.css";

interface Props {
  error: unknown;
  reset: () => void;
}

export function ErrorFallback(props: Props) {
  const message = () => {
    const e = props.error;
    if (e instanceof Error) return e.message;
    if (typeof e === "string") return e;
    return "Beklenmeyen bir hata oluştu.";
  };

  return (
    <div class={styles.container}>
      <div class={styles.icon}>⚠</div>
      <h2 class={styles.title}>Bir şeyler ters gitti</h2>
      <p class={styles.message}>{message()}</p>
      <button class={styles.retryBtn} onClick={props.reset}>
        Yeniden Dene
      </button>
    </div>
  );
}
