import { device } from "$lib/device/manager.svelte";

class configState {
  setPinDialogOpen = $state(false);
  currentPin = $state("");
  newPin = $state("");
  confirmPin = $state("");
  pinError = $state("");
  isSettingPin = $state(false);

  minPinDialogOpen = $state(false);
  minPinCurrentPin = $state("");
  minPinNewPin = $state("");
  minPinConfirmPin = $state("");
  minPinLength = $state(4);
  minPinError = $state("");

  authPinDialogOpen = $state(false);
  authPin = $state("");
  authPinError = $state("");

  dialogOpen = $state(false);
  dialogTitle = $state("");
  dialogMessage = $state("");
  dialogType: "success" | "error" = $state("success");

  async handleSave() {
    if (device.method === "FIDO" && device.fidoInfo?.options?.clientPin) {
      this.authPin = "";
      this.authPinError = "";
      this.authPinDialogOpen = true;
      return;
    }

    const result = await device.save();
    if (result) {
      this.showStatusDialog(result.success ? "Success" : "Write Failed", result.msg, result.success ? "success" : "error");
    }
  }

  async confirmAuthPinSave() {
    const result = await device.save(this.authPin);
    if (result.success) {
      this.authPinDialogOpen = false;
      this.showStatusDialog("Success", result.msg);
    } else {
      this.authPinError = result.msg as string;
    }
  }

  showStatusDialog(title: string, message: string, type: "success" | "error" = "success") {
    this.dialogTitle = title;
    this.dialogMessage = message;
    this.dialogType = type;
    this.dialogOpen = true;
  }

  openPinDialog() {
    this.pinError = "";
    this.currentPin = "";
    this.newPin = "";
    this.confirmPin = "";
    if (device.fidoInfo?.options?.clientPin) {
      this.isSettingPin = false;
    } else {
      this.isSettingPin = true;
    }
    this.setPinDialogOpen = true;
  }

  async openMinPinDialog() {
    this.minPinError = "";
    this.minPinCurrentPin = "";
    this.minPinNewPin = "";
    this.minPinConfirmPin = "";
    this.minPinLength = device.fidoInfo?.minPinLength || 4;
    this.minPinDialogOpen = true;
  }

  async handlePinChange() {
    this.pinError = "";
    if (this.newPin.length < (device.fidoInfo?.minPinLength || 4)) {
      this.pinError = `PIN must be at least ${device.fidoInfo?.minPinLength || 4} characters`;
      return;
    }
    if (this.newPin !== this.confirmPin) {
      this.pinError = "PINs do not match";
      return;
    }
    const result = await device.changePin(this.isSettingPin ? null : this.currentPin, this.newPin);
    if (result.success) {
      this.setPinDialogOpen = false;
      this.showStatusDialog("Success", this.isSettingPin ? "PIN set successfully" : "PIN changed successfully", "success");
    } else {
      this.pinError = result.msg as string;
    }
  }

  async handleMinPinChange() {
    this.minPinError = "";
    if (this.minPinLength < 4 || this.minPinLength > 63) {
      this.minPinError = "Minimum PIN length must be between 4 and 63";
      return;
    }
    if (this.minPinNewPin.length < this.minPinLength) {
      this.minPinError = `New PIN must be at least ${this.minPinLength} characters`;
      return;
    }
    if (this.minPinNewPin !== this.minPinConfirmPin) {
      this.minPinError = "New PINs do not match";
      return;
    }
    const minLengthResult = await device.updateMinPinLength(this.minPinCurrentPin, this.minPinLength);
    if (!minLengthResult.success) {
      this.minPinError = minLengthResult.msg as string;
      return;
    }
    const pinResult = await device.changePin(this.minPinCurrentPin, this.minPinNewPin);
    if (pinResult.success) {
      this.minPinDialogOpen = false;
      this.showStatusDialog(
        "Success",
        `Minimum PIN length updated to ${this.minPinLength} and PIN changed successfully`,
        "success",
      );
    } else {
      this.minPinError = pinResult.msg as string;
    }
  }
}

export const configViewState = new configState();
