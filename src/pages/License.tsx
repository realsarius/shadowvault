import { createSignal, onMount, Show } from "solid-js";
import { TbOutlineLock, TbOutlineCheck } from "solid-icons/tb";
import { Modal } from "../components/ui/Modal";
import { Button } from "../components/ui/Button";
import { api } from "../api/tauri";
import { activateLicense } from "../store";
import styles from "./License.module.css";

const FREE_LIMIT = 3;
const KEY_PATTERN = /^SV-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}$/;

function formatKeyInput(raw: string): string {
  const clean = raw.toUpperCase().replace(/[^A-Z0-9]/g, "");
  const parts: string[] = [];
  for (let i = 0; i < Math.min(clean.length, 16); i += 4) {
    parts.push(clean.slice(i, i + 4));
  }
  return parts.length > 0 ? "SV-" + parts.join("-") : "";
}

interface Props {
  open: boolean;
  onClose: () => void;
  sourceCount: number;
  subtitle?: string;
}

export function UpgradeModal(props: Props) {
  const [key, setKey] = createSignal("");
  const [hardwareId, setHardwareId] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  onMount(async () => {
    try {
      setHardwareId(await api.license.getHardwareId());
    } catch {
      setHardwareId("Alınamadı");
    }
  });

  const handleInput = (e: Event) => {
    const input = e.currentTarget as HTMLInputElement;
    const formatted = formatKeyInput(input.value);
    setKey(formatted);
    requestAnimationFrame(() => {
      input.value = formatted;
      input.setSelectionRange(formatted.length, formatted.length);
    });
    setError(null);
  };

  const handleActivate = async () => {
    const k = key().trim();
    if (!KEY_PATTERN.test(k)) {
      setError("Geçersiz format. Beklenen: SV-XXXX-XXXX-XXXX-XXXX");
      return;
    }
    setLoading(true);
    setError(null);
    const result = await activateLicense(k);
    setLoading(false);
    if (result.success) {
      props.onClose();
    } else {
      setError(result.error ?? "Aktivasyon başarısız.");
    }
  };

  const isValid = () => KEY_PATTERN.test(key().trim());

  return (
    <Modal
      open={props.open}
      onClose={props.onClose}
      title="Pro Sürüme Geç"
      footer={
        <div class={styles.footer}>
          <Button variant="ghost" onClick={props.onClose}>İptal</Button>
          <Button onClick={handleActivate} disabled={!isValid() || loading()}>
            <Show when={loading()} fallback="Aktive Et">
              <span class={styles.spinner} /> Kontrol ediliyor...
            </Show>
          </Button>
        </div>
      }
    >
      <div class={styles.limitBanner}>
        <span class={styles.limitIcon}><TbOutlineLock size={20} /></span>
        <div>
          <div class={styles.limitTitle}>Ücretsiz plan sınırına ulaştınız</div>
          <div class={styles.limitSub}>
            {props.subtitle ?? `${props.sourceCount}/${FREE_LIMIT} kaynak kullanılıyor — daha fazlası için lisans gereklidir.`}
          </div>
        </div>
      </div>

      <div class={styles.features}>
        <div class={styles.featureRow}>
          <span class={styles.check}><TbOutlineCheck size={14} /></span>
          <span>Sınırsız kaynak yedekleme</span>
        </div>
        <div class={styles.featureRow}>
          <span class={styles.check}><TbOutlineCheck size={14} /></span>
          <span>Öncelikli destek</span>
        </div>
        <div class={styles.featureRow}>
          <span class={styles.check}><TbOutlineCheck size={14} /></span>
          <span>Gelecek güncellemeler</span>
        </div>
      </div>

      <div class={styles.field}>
        <label class={styles.label}>Lisans Anahtarı</label>
        <input
          class={styles.input}
          type="text"
          placeholder="SV-XXXX-XXXX-XXXX-XXXX"
          value={key()}
          onInput={handleInput}
          maxLength={22}
          spellcheck={false}
          autocomplete="off"
          onKeyDown={(e) => e.key === "Enter" && isValid() && !loading() && handleActivate()}
        />
      </div>

      <Show when={error()}>
        <div class={styles.error}>{error()}</div>
      </Show>

      <Show when={hardwareId()}>
        <div class={styles.hwid}>
          <span class={styles.hwidLabel}>Donanım Kimliği</span>
          <code class={styles.hwidValue}>{hardwareId()}</code>
        </div>
      </Show>
    </Modal>
  );
}
