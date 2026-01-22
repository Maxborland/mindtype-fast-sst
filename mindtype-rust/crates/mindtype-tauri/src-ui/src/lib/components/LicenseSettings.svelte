<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { t } from "../i18n";

  interface LicenseStatus {
    activated: boolean;
    license_key?: string;
    plan?: string;
    expires_at?: string;
    days_remaining?: number;
    credits_remaining?: number;
  }

  let licenseStatus: LicenseStatus = $state({ activated: false });
  let licenseKey = $state("");
  let isActivating = $state(false);
  let isDeactivating = $state(false);
  let errorMessage = $state("");
  let successMessage = $state("");

  // Format validation for license key (MTXX-XXXX-XXXX-XXXX)
  const licenseKeyPattern = /^MT[A-Z0-9]{2}-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}$/;

  let isValidFormat = $derived(licenseKeyPattern.test(licenseKey.toUpperCase()));
  let maskedKey = $derived(
    licenseStatus.license_key
      ? licenseStatus.license_key.slice(0, 7) + "****-****-" + licenseStatus.license_key.slice(-4)
      : ""
  );

  onMount(async () => {
    await loadLicenseStatus();
  });

  async function loadLicenseStatus() {
    try {
      licenseStatus = await invoke<LicenseStatus>("get_license_status");
    } catch (error) {
      console.error("Failed to load license status:", error);
    }
  }

  function formatLicenseKey(input: string): string {
    // Remove any non-alphanumeric characters except hyphens
    let cleaned = input.toUpperCase().replace(/[^A-Z0-9-]/g, "");

    // Remove existing hyphens to reformat
    cleaned = cleaned.replace(/-/g, "");

    // Ensure it starts with MT
    if (cleaned.length >= 2 && !cleaned.startsWith("MT")) {
      cleaned = "MT" + cleaned.slice(2);
    }

    // Add hyphens at correct positions (MT00-0000-0000-0000)
    let formatted = "";
    for (let i = 0; i < cleaned.length && i < 16; i++) {
      if (i === 4 || i === 8 || i === 12) {
        formatted += "-";
      }
      formatted += cleaned[i];
    }

    return formatted;
  }

  function handleKeyInput(e: Event) {
    const target = e.target as HTMLInputElement;
    const formatted = formatLicenseKey(target.value);
    licenseKey = formatted;
    errorMessage = "";
  }

  async function activateLicense() {
    if (!isValidFormat) {
      errorMessage = "Invalid license key format";
      return;
    }

    isActivating = true;
    errorMessage = "";
    successMessage = "";

    try {
      const result = await invoke<boolean>("activate_license", {
        licenseKey: licenseKey.toUpperCase()
      });

      if (result) {
        successMessage = "License activated successfully!";
        await loadLicenseStatus();
        licenseKey = "";
      } else {
        errorMessage = "License validation failed. Please check your key.";
      }
    } catch (error) {
      errorMessage = String(error);
    } finally {
      isActivating = false;
    }
  }

  async function deactivateLicense() {
    if (!confirm("Are you sure you want to deactivate your license?")) {
      return;
    }

    isDeactivating = true;
    errorMessage = "";
    successMessage = "";

    try {
      await invoke("deactivate_license");
      successMessage = "License deactivated.";
      await loadLicenseStatus();
    } catch (error) {
      errorMessage = String(error);
    } finally {
      isDeactivating = false;
    }
  }

  function openBuyLicense() {
    invoke("open_url", { url: "https://mindtype.space/pricing" });
  }
</script>

<div class="license-settings">
  <h3>{$t("license_activation")}</h3>

  {#if licenseStatus.activated}
    <div class="license-status license-status--active">
      <div class="license-badge">
        <span class="badge-icon">✓</span>
        <span class="badge-text">{$t("license_active")}</span>
      </div>

      <div class="license-details">
        <div class="detail-row">
          <span class="detail-label">{$t("license_key")}:</span>
          <span class="detail-value">{maskedKey}</span>
        </div>

        {#if licenseStatus.plan}
          <div class="detail-row">
            <span class="detail-label">Plan:</span>
            <span class="detail-value plan-badge">{licenseStatus.plan}</span>
          </div>
        {/if}

        {#if licenseStatus.days_remaining !== undefined}
          <div class="detail-row">
            <span class="detail-label">Expires:</span>
            <span class="detail-value" class:expiring-soon={licenseStatus.days_remaining < 30}>
              {licenseStatus.days_remaining} days remaining
            </span>
          </div>
        {/if}

        {#if licenseStatus.credits_remaining !== undefined}
          <div class="detail-row">
            <span class="detail-label">Credits:</span>
            <span class="detail-value">{licenseStatus.credits_remaining.toLocaleString()}</span>
          </div>
        {/if}
      </div>

      <div class="license-actions">
        <button
          class="btn btn--danger"
          onclick={deactivateLicense}
          disabled={isDeactivating}
        >
          {isDeactivating ? "..." : $t("deactivate_license")}
        </button>
      </div>
    </div>
  {:else}
    <div class="license-status license-status--trial">
      <div class="license-badge trial">
        <span class="badge-icon">⏱</span>
        <span class="badge-text">{$t("trial_mode")}</span>
      </div>

      <p class="trial-info">
        Enter your license key to unlock all features.
      </p>

      <div class="license-input">
        <input
          type="text"
          value={licenseKey}
          oninput={handleKeyInput}
          placeholder="MTXX-XXXX-XXXX-XXXX"
          maxlength="19"
          class:invalid={licenseKey.length > 0 && !isValidFormat}
        />
        {#if licenseKey.length > 0 && !isValidFormat}
          <span class="format-hint">Format: MTXX-XXXX-XXXX-XXXX</span>
        {/if}
      </div>

      <div class="license-actions">
        <button
          class="btn btn--primary"
          onclick={activateLicense}
          disabled={isActivating || !isValidFormat}
        >
          {isActivating ? "..." : $t("activate")}
        </button>
        <button class="btn btn--link" onclick={openBuyLicense}>
          {$t("buy_license")}
        </button>
      </div>
    </div>
  {/if}

  {#if errorMessage}
    <div class="message message--error">{errorMessage}</div>
  {/if}

  {#if successMessage}
    <div class="message message--success">{successMessage}</div>
  {/if}
</div>

<style>
  .license-settings {
    padding: 8px 0;
  }

  h3 {
    font-size: 12px;
    margin: 0 0 12px 0;
    font-weight: bold;
  }

  .license-status {
    padding: 12px;
    border: 1px solid var(--frame-inner-dark);
    background: white;
  }

  .license-badge {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
  }

  .badge-icon {
    font-size: 16px;
  }

  .license-status--active .badge-icon {
    color: #00aa00;
  }

  .license-status--trial .badge-icon {
    color: #996600;
  }

  .badge-text {
    font-size: 12px;
    font-weight: bold;
  }

  .license-details {
    margin-bottom: 12px;
  }

  .detail-row {
    display: flex;
    justify-content: space-between;
    padding: 4px 0;
    border-bottom: 1px solid #eee;
    font-size: 11px;
  }

  .detail-row:last-child {
    border-bottom: none;
  }

  .detail-label {
    color: var(--text-secondary);
  }

  .detail-value {
    font-weight: 500;
  }

  .plan-badge {
    text-transform: uppercase;
    background: #e6f0ff;
    color: #0066cc;
    padding: 1px 6px;
    border-radius: 2px;
    font-size: 10px;
  }

  .expiring-soon {
    color: #cc6600;
  }

  .trial-info {
    font-size: 11px;
    color: var(--text-secondary);
    margin-bottom: 12px;
  }

  .license-input {
    margin-bottom: 12px;
  }

  .license-input input {
    width: 100%;
    padding: 8px;
    font-size: 12px;
    font-family: monospace;
    text-transform: uppercase;
    letter-spacing: 1px;
    background: white;
    border: 2px solid;
    border-color: var(--frame-inner-dark) var(--frame-inner-light) var(--frame-inner-light) var(--frame-inner-dark);
    box-sizing: border-box;
  }

  .license-input input:focus {
    outline: none;
    border-color: var(--accent-blue, #0066CC);
  }

  .license-input input.invalid {
    border-color: #cc0000;
  }

  .format-hint {
    display: block;
    font-size: 10px;
    color: #cc6600;
    margin-top: 4px;
  }

  .license-actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .btn {
    padding: 6px 16px;
    font-size: 11px;
    font-family: inherit;
    cursor: pointer;
    border: 2px solid;
    border-color: var(--frame-inner-light) var(--frame-inner-dark) var(--frame-inner-dark) var(--frame-inner-light);
    background: var(--button-face, #c0c0c0);
  }

  .btn:hover:not(:disabled) {
    background: var(--button-hover, #d0d0d0);
  }

  .btn:active:not(:disabled) {
    border-color: var(--frame-inner-dark) var(--frame-inner-light) var(--frame-inner-light) var(--frame-inner-dark);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn--primary {
    background: var(--accent-blue, #0066CC);
    color: white;
    border-color: #004499 #0088ff #0088ff #004499;
  }

  .btn--primary:hover:not(:disabled) {
    background: #0077dd;
  }

  .btn--danger {
    color: #cc0000;
  }

  .btn--link {
    background: transparent;
    border: none;
    color: #0066cc;
    padding: 6px 8px;
    text-decoration: underline;
  }

  .btn--link:hover {
    color: #0044aa;
  }

  .message {
    margin-top: 12px;
    padding: 8px;
    font-size: 11px;
    border: 1px solid;
  }

  .message--error {
    background: #ffe6e6;
    border-color: #cc0000;
    color: #990000;
  }

  .message--success {
    background: #e6ffe6;
    border-color: #00aa00;
    color: #006600;
  }
</style>
