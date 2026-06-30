# Coverage Report (excl. main.rs)

**2527** / **2881** lines covered — **87.71%**

**Threshold:** 80% | **Status:** ✅ PASS

| Crate | Files | Covered | Total | Coverage |
| --- | --- | --- | --- | --- |
| `aether-aggregator` | 11 | 1350 | 1509 | 89.5% |
| `aether-auth` | 3 | 175 | 189 | 92.6% |
| `aetherd` | 18 | 1002 | 1183 | 84.7% |

## `aether-aggregator`

### crates/aether-aggregator/src/storage/csi.rs

- Coverage: 🟡 **76.7%** (181/236 lines)
- **Missed lines:**
  - Line 118
  - Lines 154-155
  - Line 157
  - Line 160
  - Line 202
  - Line 225
  - Line 228
  - Line 246
  - Line 258
  - Line 280
  - Line 285
  - Line 340
  - Line 344
  - Line 348
  - Line 355
  - Line 361
  - Line 366
  - Line 370
  - Line 373
  - Line 377
  - Line 380
  - Line 384
  - Line 387
  - Line 391
  - Line 394
  - Line 398
  - Line 403
  - Line 407
  - Line 412
  - Line 416
  - Line 430
  - Line 433
  - Line 458
  - Line 465
  - Line 469
  - Line 476
  - Line 488
  - Line 491
  - Line 508
  - Line 512
  - Line 526
  - Line 529
  - Line 550
  - Line 554
  - Line 558
  - Line 565
  - Line 577
  - Line 580
  - Line 597
  - Line 601
  - Line 609
  - Line 613
  - Line 618
  - Line 622

### crates/aether-aggregator/tests/auction_convergence_timing.rs

- Coverage: 🟡 **79.0%** (83/105 lines)
- **Missed lines:**
  - Lines 43-45
  - Line 47
  - Lines 49-53
  - Lines 55-57
  - Line 59
  - Line 61
  - Line 63
  - Lines 65-67
  - Lines 69-70
  - Lines 94-95

### crates/aether-aggregator/tests/deterministic_scheduling_selection.rs

- Coverage: 🟡 **81.8%** (99/121 lines)
- **Missed lines:**
  - Lines 40-42
  - Line 44
  - Lines 46-50
  - Lines 52-54
  - Line 56
  - Line 58
  - Line 60
  - Lines 62-64
  - Lines 66-67
  - Lines 89-90

### crates/aether-aggregator/src/network/hpe_vc.rs

- Coverage: 🟡 **85.0%** (136/160 lines)
- **Missed lines:**
  - Line 182
  - Line 184
  - Lines 201-202
  - Line 207
  - Lines 245-247
  - Lines 280-282
  - Line 299
  - Lines 314-316
  - Line 333
  - Lines 339-341
  - Line 360
  - Line 364
  - Lines 376-378

### crates/aether-aggregator/src/scheduler.rs

- Coverage: 🟡 **88.1%** (59/67 lines)
- **Missed lines:**
  - Lines 82-84
  - Line 115
  - Line 133
  - Lines 137-138
  - Line 160

### crates/aether-aggregator/src/tie_breaker.rs

- Coverage: 🟢 **92.1%** (35/38 lines)
- **Missed lines:**
  - Line 48
  - Lines 51-52

### crates/aether-aggregator/tests/csi_zvol_mount_lifecycle.rs

- Coverage: 🟢 **93.3%** (263/282 lines)
- **Missed lines:**
  - Line 62
  - Line 88
  - Line 112
  - Line 138
  - Line 154
  - Line 191
  - Line 208
  - Line 219
  - Line 230
  - Line 248
  - Line 265
  - Line 275
  - Line 314
  - Line 331
  - Line 349
  - Line 369
  - Line 383
  - Line 394
  - Line 405

### crates/aether-aggregator/tests/heartbeat_timeout_tests.rs

- Coverage: 🟢 **94.3%** (50/53 lines)
- **Missed lines:**
  - Line 34
  - Line 44
  - Line 65

### crates/aether-aggregator/tests/switch_vlan_tagging_integration.rs

- Coverage: 🟢 **99.3%** (398/401 lines)
- **Missed lines:**
  - Line 324
  - Line 555
  - Line 713

### crates/aether-aggregator/src/lib.rs

- Coverage: 🟢 **100.0%** (20/20 lines)

### crates/aether-aggregator/src/registry.rs

- Coverage: 🟢 **100.0%** (26/26 lines)

## `aether-auth`

### crates/aether-auth/tests/mtls_integration_tests.rs

- Coverage: 🟡 **86.7%** (85/98 lines)
- **Missed lines:**
  - Lines 35-37
  - Lines 39-44
  - Line 68
  - Line 94
  - Line 124
  - Line 146

### crates/aether-auth/src/token.rs

- Coverage: 🟢 **97.8%** (45/46 lines)
- **Missed lines:**
  - Line 56

### crates/aether-auth/src/mtls.rs

- Coverage: 🟢 **100.0%** (45/45 lines)

## `aetherd`

### crates/aetherd/tests/vlan_isolation_verification.rs

- Coverage: 🔴 **41.7%** (15/36 lines)
- **Missed lines:**
  - Lines 10-12
  - Lines 14-26
  - Lines 28-29
  - Line 31
  - Lines 34-35

### crates/aetherd/src/cloud_init.rs

- Coverage: 🔴 **67.6%** (48/71 lines)
- **Missed lines:**
  - Line 53
  - Lines 57-58
  - Lines 105-110
  - Line 135
  - Lines 140-145
  - Lines 147-149
  - Lines 151-152
  - Lines 156-157

### crates/aetherd/src/hypervisor/firecracker.rs

- Coverage: 🟡 **74.2%** (66/89 lines)
- **Missed lines:**
  - Line 138
  - Lines 158-174
  - Line 177
  - Line 184
  - Line 213
  - Line 262
  - Line 273

### crates/aetherd/src/hypervisor/qemu.rs

- Coverage: 🟡 **76.6%** (105/137 lines)
- **Missed lines:**
  - Line 67
  - Line 125
  - Line 144
  - Lines 147-148
  - Line 152
  - Lines 163-169
  - Lines 171-172
  - Lines 177-178
  - Lines 183-185
  - Lines 189-190
  - Line 193
  - Lines 232-233
  - Lines 253-255
  - Line 281
  - Lines 289-290
  - Line 296

### crates/aetherd/tests/cloud_init_iso_guest_boot.rs

- Coverage: 🟡 **77.4%** (24/31 lines)
- **Missed lines:**
  - Line 25
  - Line 42
  - Lines 49-53

### crates/aetherd/src/telemetry.rs

- Coverage: 🟡 **79.0%** (83/105 lines)
- **Missed lines:**
  - Line 151
  - Line 153
  - Lines 200-209
  - Line 214
  - Lines 238-239
  - Line 247
  - Lines 249-250
  - Line 255
  - Lines 257-258
  - Line 263

### crates/aetherd/src/lib.rs

- Coverage: 🟡 **81.1%** (77/95 lines)
- **Missed lines:**
  - Line 149
  - Lines 181-184
  - Lines 192-193
  - Line 195
  - Lines 199-200
  - Lines 206-209
  - Lines 211-212
  - Line 215
  - Line 263

### crates/aetherd/src/storage/zfs.rs

- Coverage: 🟡 **81.9%** (59/72 lines)
- **Missed lines:**
  - Line 249
  - Line 251
  - Line 263
  - Line 265
  - Line 270
  - Line 272
  - Line 297
  - Line 299
  - Line 308
  - Line 310
  - Lines 341-343

### crates/aetherd/src/bidder.rs

- Coverage: 🟡 **89.3%** (25/28 lines)
- **Missed lines:**
  - Line 49
  - Line 62
  - Line 92

### crates/aetherd/tests/firecracker_vm_boot_lifecycle.rs

- Coverage: 🟡 **89.5%** (34/38 lines)
- **Missed lines:**
  - Line 24
  - Line 30
  - Line 37
  - Line 53

### crates/aetherd/tests/node_tests.rs

- Coverage: 🟢 **93.9%** (168/179 lines)
- **Missed lines:**
  - Line 33
  - Line 50
  - Line 78
  - Line 114
  - Line 132
  - Line 144
  - Line 176
  - Line 186
  - Line 200
  - Line 236
  - Line 248

### crates/aetherd/tests/zfs_thin_provisioning_limits.rs

- Coverage: 🟢 **94.3%** (50/53 lines)
- **Missed lines:**
  - Line 18
  - Line 39
  - Line 67

### crates/aetherd/tests/vsock_stream_performance.rs

- Coverage: 🟢 **98.4%** (60/61 lines)
- **Missed lines:**
  - Line 93

### crates/aetherd/src/network/bridge.rs

- Coverage: 🟢 **100.0%** (12/12 lines)

### crates/aetherd/src/vsock.rs

- Coverage: 🟢 **100.0%** (79/79 lines)

### crates/aetherd/tests/bidding_resource_thresholds.rs

- Coverage: 🟢 **100.0%** (34/34 lines)

### crates/aetherd/tests/host_system_metrics_query.rs

- Coverage: 🟢 **100.0%** (15/15 lines)

### crates/aetherd/tests/qemu_kvm_vm_lifecycle.rs

- Coverage: 🟢 **100.0%** (48/48 lines)
