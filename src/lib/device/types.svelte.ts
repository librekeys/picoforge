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

export interface DeviceConfigInput {
  vid?: string;
  pid?: string;
  productName?: string;
  ledGpio?: number;
  ledBrightness?: number;
  touchTimeout?: number;
  ledDriver?: number;
  ledDimmable?: boolean;
  powerCycleOnReset?: boolean;
  ledSteady?: boolean;
  enableSecp256k1?: boolean;
}

export interface DeviceInfo {
  serial: string;
  flashUsed: number;
  flashTotal: number;
  firmwareVersion: string;
}

export interface FullDeviceStatus {
  info: DeviceInfo;
  config: DeviceConfig;
  secureBoot: boolean;
  secureLock: boolean;
  method: string;
}

export interface SecurityState {
  secureBoot: boolean;
  secureLock: boolean;
  confirmed: boolean;
}

export interface FidoInfo {
  versions: string[];
  extensions: string[];
  aaguid: string;
  options: Record<string, boolean>;
  maxMsgSize: number;
  pinProtocols: number[];
  // remainingDiscCreds: number;
  minPinLength: number;
  firmwareVersion: string;
}

export interface StoredCredential {
  credentialId: string;
  rpId: string;
  rpName: string;
  userId: string;
  userName: string;
  userDisplayName: string;
}
