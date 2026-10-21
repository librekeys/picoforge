# Picoforge 看不到 RS-Key 的 FIDO2 —— 根因与修复

## 症状
- RS-Key（VID:PID = 1050:0407，RP2350/16MB）在浏览器里作为安全密钥可用。
- Picoforge 却显示 "FIDO information not available" / "FIDO Passkeys are not
  supported on this device"，即 FIDO2 不可用。
- PC/SC（Rescue）通道正常：能读到固件 8.6、序列号、`supported=0x023B`
  （含 FIDO2 位）。

## 根因（已复现并定位）
Windows 把该 FIDO HID 接口（`HID\VID_1050&PID_0407&MI_01`,
`HID_DEVICE_UP:F1D0_U:0001`）绑定到系统内置 `fidohid` 驱动，其设备安全描述符为：

```
D:P(A;;GA;;;BA)(A;;GA;;;SY)(A;;GA;;;S-1-5-80-242729624-...-2225943459)
```

只授予 Administrators(BA) / SYSTEM(SY) / NT SERVICE\CryptSvc。普通权限进程
`CreateFile` 该接口得到 **ERROR_ACCESS_DENIED (5)**。

hidapi 的 `hid_enumerate` 会以只读方式打开每个 HID 接口，打不开的直接跳过，
于是 FIDO 接口（Usage Page 0xF1D0）根本不出现在枚举结果里；
`HidTransport::open()` 找不到设备 → `get_fido_info()` 失败 → fido_info=None →
UI 报 FIDO 不可用。

证据：
- 普通权限 `hidapi` 枚举 F1D0 条目 = 0；提权后 = 1。
- `temp/hidlist/` 是复现/验证用小程序（直接 CreateFile 与 hidapi 枚举对比）。
- picoforge 日志（`%LOCALAPPDATA%\suyogtandel\picoforge\data\logs\picoforge.log`）：
  普通权限为 `No FIDO device found with Usage Page 0xF1D0`；
  提权后 `Found FIDO device: VendorID=0x1050, ProductID=0x0407` 且 GetInfo 成功。

## 修复方案

### 方案 A（临时、零改动）：以管理员身份运行 picoforge
右键 exe → 以管理员身份运行，或对快捷方式勾选“以管理员身份运行”。

### 方案 B（永久、可普通权限运行）：放开 FIDO 设备 ACL
以管理员运行 `fix_fido_acl.ps1`：给所有在线的 FIDO HID devnode 追加
`(A;;GA;;;BU)`（Builtin Users 完全访问），然后 `pnputil /restart-device`
重启该设备（或拔插一次）。之后普通权限进程即可打开 FIDO 接口。

验证：修复后普通权限 `hidapi` 能枚举到 F1D0 条目；普通权限启动 picoforge，
日志出现 `Found FIDO device` 且 GetInfo 成功。

> 注意：`fidohid.inf` 重新安装/驱动重装后 SD 可能被重置，重跑方案 B 即可。
