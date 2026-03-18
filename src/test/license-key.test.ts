import { describe, expect, it } from "vitest";
import { formatLicenseKeyInput, LICENSE_KEY_PATTERN } from "../utils/licenseKey";

describe("license key input formatter", () => {
  it("keeps a fully valid key unchanged when pasted", () => {
    const key = "SV-URBQ-W6SA-ZUT7-7EB4";
    expect(formatLicenseKeyInput(key)).toBe(key);
  });

  it("formats keys pasted without dashes", () => {
    const key = "svurbqw6sazut77eb4";
    expect(formatLicenseKeyInput(key)).toBe("SV-URBQ-W6SA-ZUT7-7EB4");
  });

  it("filters invalid characters and limits to 16 body chars", () => {
    const key = "SV-ABCD-EFGH-IJKL-MNOP-QQQQ";
    expect(formatLicenseKeyInput(key)).toBe("SV-ABCD-EFGH-IJKL-MNOP");
  });

  it("matches activation pattern only when complete", () => {
    expect(LICENSE_KEY_PATTERN.test("SV-ABCD-EFGH-IJKL-MNOP")).toBe(true);
    expect(LICENSE_KEY_PATTERN.test("SV-ABCD-EFGH-IJKL")).toBe(false);
  });
});
