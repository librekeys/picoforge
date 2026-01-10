import { invoke } from "@tauri-apps/api/core";
import { logger } from "$lib/utils/log.svelte";
import { DEFAULT_CONFIG, DEFAULT_DEVICE_INFO, VENDORS } from "$lib/device/constants.svelte";
import type { DeviceConfig, DeviceInfo, FidoInfo, SecurityState } from "$lib/device/types.svelte";

class DeviceManager {
  interfaceType: "rescue" | "fido" | null = $state(null);
  loading = $state(false);
  connected = $state(false);
  fidoInfo: FidoInfo | null = $state(null);

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
    try {
      logger.add("Attempting to connect to device...", "info");

      const status: any = await invoke("refresh_device_status");
      
      this.interfaceType = (status.config.vid && status.config.vid !== "" && status.config.vid !== "0000") ? "rescue" : "fido";

      this.info = {
        serial: status.info.serial,
        flashUsed: status.info.flash_used,
        flashTotal: status.info.flash_total,
        firmwareVersion: status.info.firmware_version,
      };

      this.config = {
        vid: status.config.vid || "1209",
        pid: status.config.pid || "0001",
        productName: status.config.product_name || "FIDO Device",
        ledGpio: status.config.led_gpio || 0,
        ledBrightness: status.config.led_brightness || 0,
        touchTimeout: status.config.touch_timeout || 0,
        ledDimmable: status.config.led_dimmable || false,
        powerCycleOnReset: status.config.power_cycle_on_reset || false,
        ledSteady: status.config.led_steady || false,
        enableSecp256k1: status.config.enable_secp256k1 || false,
        ledDriver: status.config.led_driver ? String(status.config.led_driver) : "1",
      };

      this.#originalConfig = JSON.parse(JSON.stringify(this.config));

      this.security = {
        secureBoot: status.secure_boot,
        secureLock: status.secure_lock,
        confirmed: false,
      };

      try {
        const fido: any = await invoke("get_fido_info");
        this.fidoInfo = fido;
        console.log("FIDO Info:", fido);
        console.log("Min PIN Length:", fido.minPinLength);
        logger.add(`FIDO Info Retrieved: ${fido.versions?.join(', ')}`, "info");
      } catch (fidoErr) {
        logger.add(`Could not fetch FIDO info: ${fidoErr}`, "warning");
        this.fidoInfo = null;
      }

      if (!this.connected) {
        const modeText = this.fidoInfo ? "RESCUE + FIDO" : this.interfaceType?.toUpperCase() || "UNKNOWN";
        logger.add(
          `Connected via ${modeText}! Serial: ${this.info.serial}`,
          "success",
        );
      }
      this.connected = true;
    } catch (err) {
      console.error("Connection failed:", err);
      if (this.connected) {
        logger.add(`Connection lost: ${err}`, "error");
      }
      this.connected = false;
      this.fidoInfo = null;
    } finally {
      this.loading = false;
    }
  }

  async save() {
    if (!this.connected || !this.#originalConfig) return { success: false, msg: "Device not connected" };

    this.loading = true;
    logger.add(`Analyzing changes for ${this.interfaceType} interface...`, "info");

    try {
      const rustConfig: any = {};
      const command = this.interfaceType === "fido" ? "write_fido_config" : "write_config";

      if (this.config.vid !== this.#originalConfig.vid || this.config.pid !== this.#originalConfig.pid) {
        rustConfig.vid = this.config.vid;
        rustConfig.pid = this.config.pid;
        logger.add(`Queuing change: VID/PID -> ${this.config.vid}:${this.config.pid}`, "info");
      }

      if (this.config.productName !== this.#originalConfig.productName) {
        rustConfig.product_name = this.config.productName;
        logger.add(`Queuing change: Product Name -> ${this.config.productName}`, "info");
      }

      if (Number(this.config.ledGpio) !== Number(this.#originalConfig.ledGpio)) {
        rustConfig.led_gpio = Number(this.config.ledGpio);
        logger.add(`Queuing change: LED GPIO -> ${this.config.ledGpio}`, "info");
      }

      if (Number(this.config.ledBrightness) !== Number(this.#originalConfig.ledBrightness)) {
        rustConfig.led_brightness = Number(this.config.ledBrightness);
        logger.add(`Queuing change: Brightness -> ${this.config.ledBrightness}`, "info");
      }

      if (Number(this.config.touchTimeout) !== Number(this.#originalConfig.touchTimeout)) {
        rustConfig.touch_timeout = Number(this.config.touchTimeout);
        logger.add(`Queuing change: Timeout -> ${this.config.touchTimeout}`, "info");
      }

      const optionsChanged = this.config.ledDimmable !== this.#originalConfig.ledDimmable ||
        this.config.powerCycleOnReset !== this.#originalConfig.powerCycleOnReset ||
        this.config.ledSteady !== this.#originalConfig.ledSteady;

      if (optionsChanged) {
        rustConfig.led_dimmable = this.config.ledDimmable;
        rustConfig.power_cycle_on_reset = this.config.powerCycleOnReset;
        rustConfig.led_steady = this.config.ledSteady;
        logger.add("Queuing change: Device Options (Bitmask updated)", "info");
      }

      if (this.config.enableSecp256k1 !== this.#originalConfig.enableSecp256k1) {
        rustConfig.enable_secp256k1 = this.config.enableSecp256k1;
        logger.add(`Queuing change: Secp256k1 -> ${this.config.enableSecp256k1}`, "info");
      }

      if (Number(this.config.ledDriver) !== Number(this.#originalConfig.ledDriver)) {
        rustConfig.led_driver = Number(this.config.ledDriver);
        logger.add(`Queuing change: LED Driver -> ${this.config.ledDriver}`, "info");
      }

      if (Object.keys(rustConfig).length === 0) {
        logger.add("No changes detected.", "warning");
        return { success: false, msg: "No changes detected." };
      } else {
        logger.add("Sending configuration to device...", "info");
        const response = await invoke(command, { config: rustConfig });
        logger.add(`Device Response: ${response}`, "success");

        await this.refresh();
        return { success: true, msg: "Configuration Applied Successfully!" };
      }
    } catch (err: any) {
      logger.add(`Write Failed: ${err}`, "error");
      return { success: false, msg: "Error: " + err };
    } finally {
      this.loading = false;
    }
  }

  setVendor(value: string) {
    const v = VENDORS.find((x) => x.value === value);
    if (v && value !== "custom") {
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
}

export const device = new DeviceManager();
