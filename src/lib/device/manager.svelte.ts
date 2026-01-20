import { invoke } from "@tauri-apps/api/core";
import { logger } from "$lib/services/log.svelte";
import { DEFAULT_CONFIG, DEFAULT_DEVICE_INFO, VENDORS } from "$lib/device/constants.svelte";
import type {
  DeviceConfig,
  DeviceInfo,
  FidoInfo,
  SecurityState,
  FullDeviceStatus,
  DeviceConfigInput,
  StoredCredential,
} from "$lib/device/types.svelte";

class DeviceManager {
  loading = $state(false);
  connected = $state(false);
  fidoInfo: FidoInfo | null = $state(null);
  error: string | null = $state(null);
  method: string = $state("");

  credentials: StoredCredential[] = $state([]);
  unlocked = $state(false);

  config: DeviceConfig = $state({ ...DEFAULT_CONFIG });
  info: DeviceInfo = $state({ ...DEFAULT_DEVICE_INFO });
  security: SecurityState = $state({
    secureBoot: false,
    secureLock: false,
    confirmed: false,
  });

  // Internal state for diffing
  #originalConfig: any = null;

  get selectedVendor() {
    if (!this.connected) return "custom";
    const match = VENDORS.find((v) => v.vid === this.config.vid && v.pid === this.config.pid);
    return match ? match.value : "custom";
  }

  async refresh() {
    this.loading = true;
    this.error = null;
    try {
      logger.add("Attempting to connect to device...", "info");

      const status = await invoke<FullDeviceStatus>("read_device_details");

      this.info = status.info;

      this.config = {
        ...status.config,
        ledDriver: status.config.ledDriver ? String(status.config.ledDriver) : "1",
      };

      this.#originalConfig = JSON.parse(JSON.stringify(this.config));

      this.security = {
        secureBoot: status.secureBoot,
        secureLock: status.secureLock,
        confirmed: false,
      };

      this.method = status.method;

      const fido = await invoke<FidoInfo>("get_fido_info");

      this.fidoInfo = fido;

      if (!this.connected) {
        logger.add(`Device Connected! Serial: ${this.info.serial}, FW: v${this.info.firmwareVersion}`, "success");
      }
      this.connected = true;
    } catch (err: any) {
      console.error("Connection failed:", err);

      // Handle structured error from Rust (PFError)
      if (err && typeof err === "object" && err.type === "NoDevice") {
        this.error = null;
        this.connected = false;
        // Don't log "No device" as an error to the user log system, 
        // it's a normal state when nothing is plugged in.
      } else {
        const msg = typeof err === "string" ? err : err.message || JSON.stringify(err);
        this.error = msg;
        if (this.connected) {
          logger.add(`Connection lost: ${msg}`, "error");
        } else {
          logger.add(`Connection failed: ${msg}`, "error");
        }
        this.connected = false;
      }
    } finally {
      this.loading = false;
    }
  }

  async save() {
    if (!this.connected || !this.#originalConfig) return { success: false, msg: "Device not connected" };

    this.loading = true;
    logger.add("Analyzing configuration changes...", "info");

    try {
      const rustConfig: DeviceConfigInput = {};

      // Diffing logic
      if (this.config.vid !== this.#originalConfig.vid || this.config.pid !== this.#originalConfig.pid) {
        rustConfig.vid = this.config.vid;
        rustConfig.pid = this.config.pid;
        logger.add(`Queuing change: VID/PID -> ${this.config.vid}:${this.config.pid}`, "info");
      }

      if (this.config.productName !== this.#originalConfig.productName) {
        rustConfig.productName = this.config.productName;
        logger.add(`Queuing change: Product Name -> ${this.config.productName}`, "info");
      }

      if (Number(this.config.ledGpio) !== Number(this.#originalConfig.ledGpio)) {
        rustConfig.ledGpio = Number(this.config.ledGpio);
        logger.add(`Queuing change: LED GPIO -> ${this.config.ledGpio}`, "info");
      }

      if (Number(this.config.ledBrightness) !== Number(this.#originalConfig.ledBrightness)) {
        rustConfig.ledBrightness = Number(this.config.ledBrightness);
        logger.add(`Queuing change: Brightness -> ${this.config.ledBrightness}`, "info");
      }

      if (Number(this.config.touchTimeout) !== Number(this.#originalConfig.touchTimeout)) {
        rustConfig.touchTimeout = Number(this.config.touchTimeout);
        logger.add(`Queuing change: Timeout -> ${this.config.touchTimeout}`, "info");
      }

      const optionsChanged =
        this.config.ledDimmable !== this.#originalConfig.ledDimmable ||
        this.config.powerCycleOnReset !== this.#originalConfig.powerCycleOnReset ||
        this.config.ledSteady !== this.#originalConfig.ledSteady;

      if (optionsChanged) {
        rustConfig.ledDimmable = this.config.ledDimmable;
        rustConfig.powerCycleOnReset = this.config.powerCycleOnReset;
        rustConfig.ledSteady = this.config.ledSteady;
        logger.add("Queuing change: Device Options (Bitmask updated)", "info");
      }

      if (this.config.enableSecp256k1 !== this.#originalConfig.enableSecp256k1) {
        rustConfig.enableSecp256k1 = this.config.enableSecp256k1;
        logger.add(`Queuing change: Secp256k1 -> ${this.config.enableSecp256k1}`, "info");
      }

      if (Number(this.config.ledDriver) !== Number(this.#originalConfig.ledDriver)) {
        rustConfig.ledDriver = Number(this.config.ledDriver);
        logger.add(`Queuing change: LED Driver -> ${this.config.ledDriver}`, "info");
      }

      if (Object.keys(rustConfig).length === 0) {
        logger.add("No changes detected.", "warning");
        return { success: false, msg: "No changes detected." };
      } else {
        logger.add("Sending configuration to device...", "info");
        const response = await invoke("write_config", { config: rustConfig });
        logger.add(`Device Response: ${response}`, "success");

        await this.refresh();
        return { success: true, msg: "Configuration Applied Successfully!" };
      }
    } catch (err: any) {
      logger.add(`Write Failed: ${err}`, "error");
      return { success: false, msg: `Error: ${err}` };
    } finally {
      this.loading = false;
    }
  }

  setVendor(value: string) {
    const v = VENDORS.find((x) => x.value === value);
    if (v) {
      this.config.vid = v.vid;
      this.config.pid = v.pid;
      logger.add(`Selected vendor preset: ${v.label}`, "info");
    }
  }

  async changePin(current: string | null, next: string) {
    try {
      const res = await invoke("change_fido_pin", { currentPin: current, newPin: next });
      logger.add(res as string, "success");
      return { success: true };
    } catch (err) {
      logger.add(`PIN Error: ${err}`, "error");
      return { success: false, msg: err };
    }
  }

  async updateMinPinLength(currentPin: string, length: number) {
    try {
      const res = await invoke("set_min_pin_length", { currentPin, minPinLength: length });
      logger.add(res as string, "success");
      await this.refresh();
      return { success: true };
    } catch (err) {
      logger.add(`Min PIN Error: ${err}`, "error");
      return { success: false, msg: err };
    }
  }

  async getCredentials(pin: string): Promise<{ success: boolean; data?: StoredCredential[]; msg?: string }> {
    try {
      logger.add("Fetching credentials...", "info");
      const creds = await invoke<StoredCredential[]>("get_credentials", { pin });

      this.credentials = creds;
      this.unlocked = true;

      logger.add(`Retrieved ${creds.length} credentials.`, "success");
      return { success: true, data: creds };
    } catch (err: any) {
      logger.add(`Failed to fetch credentials: ${err}`, "error");
      return { success: false, msg: err };
    }
  }

  lock() {
    this.credentials = [];
    this.unlocked = false;
  }

  async deleteCredential(pin: string, credentialId: string): Promise<{ success: boolean; msg?: string }> {
    try {
      logger.add("Deleting credential...", "info");
      const res = await invoke<string>("delete_credential", { pin, credentialId });
      logger.add(res, "success");
      return { success: true, msg: res };
    } catch (err: any) {
      logger.add(`Failed to delete credential: ${err}`, "error");
      return { success: false, msg: err };
    }
  }
}

export const device = new DeviceManager();
