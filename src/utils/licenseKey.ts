export const LICENSE_KEY_PATTERN = /^SV-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}$/;

export function formatLicenseKeyInput(raw: string): string {
  const clean = raw.toUpperCase().replace(/[^A-Z0-9]/g, "");
  const withoutPrefix = clean.startsWith("SV") ? clean.slice(2) : clean;
  const body = withoutPrefix.slice(0, 16);

  if (!body) return "";

  const parts: string[] = [];
  for (let i = 0; i < body.length; i += 4) {
    parts.push(body.slice(i, i + 4));
  }

  return `SV-${parts.join("-")}`;
}
