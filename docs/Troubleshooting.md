# Troubleshooting

## 1. My key is not detected by picoforge or picoforge displays a device status of "online - fido" and there are some settings that I cannot configure.

The pico-fido firmware uses by default "generic" USB Vendor ID (VID) and Product ID (PID) that are not not registered IDs and so not known by pcsc-lite. 
This means that if you flash on your key a stock pico-fido firmware it will not be recognized by pcsc-lite.  
Starting with version 0.3.0 picoforge implements a fallback, with limited functionalites, to connect to the key via the fido protocol instead of pcsc to set configuration parameters. 
When this happens the device status on the lower left corner of the application will show an orange badge with the indication "Online - fido" instead of the green "Online" badge. 

To avoid this issue we plan to obtain a valid VID/PID that can freely be used by the open source community and to communicate it to pcsc-lite team so that they can include it in their CCID driver. 
But for now there are 3 ways to work around this issue.

### 1.1. Use the fido fallback implemented in picoforge to update the VID and PID
When connected to the key with the fido fallback there are some limitations. Only a limited set of configuration parameters can be read or written.
While in this mode it is possible to change the VID/PID to use the ones of a known vendor.
But note that **a security pin needs to be set** before being able to change the configuration when in fido only mode. 
Then, after unplugging and re-plugging the key for the change to be taken into account, the key should be correctly detected by pcsc and you will be able to view and modify to the full set of configuration parameters.
**Be mindfull of the legal implications when you change the VID:PID on a key that you plan to distribute to somebody else**. You will probably want to set it back to the generic VID:PID before you distribute it.

### 1.2. Flash a firmware that you built from source with USB VID/PID known by pcsc-lite
Legaly speaking, we cannot distribute the pico-fido firmware with USB VID and PID that we do not own. And it is quite expensive to register a USB vendorID. 
This is why the pico-fido firmwares that we build and publish on github are using the so-called "generic" VID/PID.
But if you build a firmware from the source code for your own use you can choose to build it with known VID/PID that will be recognized by pcsc-lite. 
Refer to the the pico-fido firmware documentation for how to do that.

### 1.3. Add the "generic" USB VID/PID of pico-fido to the pcsc-lite CCID driver installed on your computer
There is a workwround to have pcsc-lite recorgnize a key that is using the "generic" VID/PID :  

#### On a Linux distribution where you can modify the files installed by system packages
You can manually add the generic VID and PID to the CCID driver Info.plist file.
Depending on your distribution this file may be located in a path like this : 
- /usr/lib64/pcsc/drivers/ifd-ccid.bundle/Contents/Info.plist (Fedora)
- /usr/lib/pcsc/drivers/ifd-ccid.bundle/Contents/Info.plist (Debian)

Once you have located this file you need to modify it like below :

1. Add the VID **at the end** of the `ifdVendorID` array
  ```xml
  <key>ifdVendorID</key>
  <array>
      ...
      <string>0xFEFF</string>
  </array>
  ```
2. Add the PID **at the end** of the `ifdProductID` array
  ```xml
  <key>ifdProductID</key>
  <array>
      ...
      <string>0xFCFD</string>
  </array>
  ```
3. Add a friendly name **at the end** of the `ifdFriendlyName` array
  ```xml
  <key>ifdFriendlyName</key>
  <array>
      ... 
      <string>Pico Key</string>
  </array>
  ```
4. Restart the pcsc daemon
  ```bash
  sudo systemctl restart pcscd
  ```

#### On NixOS
On NixOS the Info.plist is located in the nix store that is immutable. So it cannot be manually edited.
To pach the file you can add the following to your /etc/nixos/configuration.nix file : 
```nix
  # Override the pcscd plugin list to use your patched CCID driver with pico-fido firmware generic VID/PID
  services.pcscd.plugins = pkgs.lib.mkForce [
    (pkgs.ccid.overrideAttrs (oldAttrs: {
      nativeBuildInputs = (oldAttrs.nativeBuildInputs or []) ++ [ pkgs.xmlstarlet ];
      postInstall = (oldAttrs.postInstall or "") + ''
        plist="$out/pcsc/drivers/ifd-ccid.bundle/Contents/Info.plist"
        xmlstarlet ed -L -s "//key[text()='ifdVendorID']/following-sibling::array[1]" -t elem -n string -v "0xFEFF" "$plist"
        xmlstarlet ed -L -s "//key[text()='ifdProductID']/following-sibling::array[1]" -t elem -n string -v "0xFCFD" "$plist"
        xmlstarlet ed -L -s "//key[text()='ifdFriendlyName']/following-sibling::array[1]" -t elem -n string -v "Pico Key" "$plist"
      '';
    }))
  ];
```

#### On other immutable Linux distributions
We don't know of a way to do that on other immutable Linux distributions.
