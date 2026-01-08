export interface DeviceConfig {
  vid: string;
  pid: string;
  productName: string;
  ledGpio: number;
  ledBrightness: number;
  touchTimeout: number;
  ledDimmable: boolean;
  powerCycleOnReset: boolean;
  ledSteady: boolean;
  enableSecp256k1: boolean;
  ledDriver: string;
}

export interface DeviceInfo {
  serial: string;
  flashUsed: number;
  flashTotal: number;
  firmwareVersion: string;
}

export interface SecurityState {
  secureBoot: boolean;
  secureLock: boolean;
  confirmed: boolean;
}
