# Coverage Report (excl. main.rs)

**3223** / **3696** lines covered — **87.2%**

**Threshold:** 80% | **Status:** ✅ PASS

| Crate | Files | Covered | Total | Coverage |
| --- | --- | --- | --- | --- |
| `aether-aggregator` | 11 | 1390 | 1535 | 90.6% |
| `aether-auth` | 3 | 175 | 189 | 92.6% |
| `aetherd` | 25 | 1658 | 1972 | 84.1% |

## `aether-aggregator`

### crates/aether-aggregator/tests/auction_convergence_timing.rs

- Coverage: 🟡 **70.3%** (83/118 lines)
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
  - Lines 72-74
  - Lines 76-77
  - Line 79
  - Line 82
  - Lines 84-86
  - Line 88
  - Line 90
  - Line 92
  - Lines 116-117

### crates/aether-aggregator/tests/deterministic_scheduling_selection.rs

- Coverage: 🟡 **73.9%** (99/134 lines)
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
  - Lines 69-71
  - Lines 73-74
  - Line 76
  - Line 79
  - Lines 81-83
  - Line 85
  - Line 87
  - Line 89
  - Lines 111-112

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

### crates/aether-aggregator/src/storage/csi.rs

- Coverage: 🟢 **93.6%** (221/236 lines)
- **Missed lines:**
  - Line 157
  - Line 458
  - Line 465
  - Line 469
  - Line 508
  - Line 512
  - Line 550
  - Line 554
  - Line 558
  - Line 597
  - Line 601
  - Line 609
  - Line 613
  - Line 618
  - Line 622

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

### crates/aetherd/src/storage/iscsi.rs

- Coverage: 🔴 **0.0%** (0/25 lines)
- **Missed lines:**
  - Lines 34-36
  - Line 38
  - Lines 40-41
  - Lines 44-45
  - Line 47
  - Line 50
  - Line 53
  - Lines 55-56
  - Line 58
  - Line 62
  - Lines 65-67
  - Lines 69-70
  - Lines 73-75
  - Lines 77-78

### crates/aetherd/src/migration/memory.rs

- Coverage: 🔴 **25.0%** (5/20 lines)
- **Missed lines:**
  - Lines 31-32
  - Line 35
  - Lines 38-40
  - Lines 42-43
  - Lines 45-50
  - Line 56

### crates/aetherd/src/migration/socket.rs

- Coverage: 🔴 **49.6%** (64/129 lines)
- **Missed lines:**
  - Lines 65-69
  - Line 72
  - Line 75
  - Lines 77-80
  - Line 82
  - Line 87
  - Line 97
  - Lines 100-105
  - Lines 108-110
  - Lines 112-113
  - Line 172
  - Lines 178-180
  - Line 182
  - Lines 185-186
  - Lines 191-194
  - Line 196
  - Lines 199-202
  - Line 204
  - Line 207
  - Line 210
  - Lines 212-213
  - Lines 217-223
  - Lines 226-228
  - Line 301
  - Lines 308-309
  - Line 421
  - Line 423
  - Lines 429-431
  - Line 433

### crates/aetherd/src/migration/converge.rs

- Coverage: 🔴 **57.1%** (4/7 lines)
- **Missed lines:**
  - Lines 26-28

### crates/aetherd/src/lib.rs

- Coverage: 🔴 **60.5%** (89/147 lines)
- **Missed lines:**
  - Line 154
  - Lines 186-191
  - Lines 193-196
  - Line 198
  - Lines 203-204
  - Line 206
  - Lines 214-215
  - Line 217
  - Lines 221-222
  - Lines 228-231
  - Lines 233-234
  - Line 237
  - Line 247
  - Line 301
  - Line 309
  - Line 328
  - Lines 332-335
  - Lines 337-340
  - Lines 342-343
  - Line 345
  - Line 350
  - Lines 354-357
  - Lines 360-364
  - Lines 367-370
  - Line 372
  - Line 374

### crates/aetherd/src/telemetry.rs

- Coverage: 🟡 **76.2%** (64/84 lines)
- **Missed lines:**
  - Lines 168-177
  - Line 182
  - Lines 206-207
  - Line 215
  - Lines 217-218
  - Line 223
  - Lines 225-226
  - Line 231

### crates/aetherd/tests/cloud_init_iso_guest_boot.rs

- Coverage: 🟡 **77.4%** (24/31 lines)
- **Missed lines:**
  - Line 25
  - Line 42
  - Lines 49-53

### crates/aetherd/src/hypervisor/qemu.rs

- Coverage: 🟡 **77.6%** (159/205 lines)
- **Missed lines:**
  - Line 127
  - Line 188
  - Line 192
  - Line 242
  - Lines 244-245
  - Lines 247-248
  - Line 252
  - Lines 297-298
  - Line 300
  - Lines 304-305
  - Lines 309-310
  - Lines 312-313
  - Lines 344-350
  - Lines 352-353
  - Lines 358-359
  - Lines 364-366
  - Lines 370-371
  - Line 374
  - Lines 413-414
  - Lines 435-437
  - Line 460
  - Lines 468-469
  - Line 475
  - Lines 479-480

### crates/aetherd/src/migration/mod.rs

- Coverage: 🟡 **81.3%** (87/107 lines)
- **Missed lines:**
  - Line 109
  - Line 118
  - Line 123
  - Lines 128-129
  - Lines 133-134
  - Line 152
  - Line 159
  - Lines 172-173
  - Lines 180-183
  - Lines 207-208
  - Line 215
  - Lines 218-219

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

### crates/aetherd/tests/firecracker_vm_boot_lifecycle.rs

- Coverage: 🟡 **89.5%** (34/38 lines)
- **Missed lines:**
  - Line 24
  - Line 30
  - Line 37
  - Line 53

### crates/aetherd/src/bidder.rs

- Coverage: 🟡 **89.7%** (26/29 lines)
- **Missed lines:**
  - Line 49
  - Line 62
  - Line 92

### crates/aetherd/src/cloud_init.rs

- Coverage: 🟢 **93.2%** (69/74 lines)
- **Missed lines:**
  - Line 75
  - Lines 79-80
  - Lines 164-165

### crates/aetherd/tests/zfs_thin_provisioning_limits.rs

- Coverage: 🟢 **94.3%** (50/53 lines)
- **Missed lines:**
  - Line 18
  - Line 39
  - Line 67

### crates/aetherd/tests/node_tests.rs

- Coverage: 🟢 **94.4%** (184/195 lines)
- **Missed lines:**
  - Line 35
  - Line 52
  - Line 85
  - Line 126
  - Line 144
  - Line 156
  - Line 193
  - Line 203
  - Line 217
  - Line 258
  - Line 270

### crates/aetherd/src/hypervisor/firecracker.rs

- Coverage: 🟢 **95.6%** (87/91 lines)
- **Missed lines:**
  - Lines 178-179
  - Line 183
  - Line 280

### crates/aetherd/tests/vlan_isolation_verification.rs

- Coverage: 🟢 **96.9%** (62/64 lines)
- **Missed lines:**
  - Line 29
  - Line 31

### crates/aetherd/tests/migration_tests.rs

- Coverage: 🟢 **97.4%** (333/342 lines)
- **Missed lines:**
  - Lines 28-29
  - Line 162
  - Line 202
  - Line 227
  - Line 252
  - Lines 417-418
  - Line 500

### crates/aetherd/tests/vsock_stream_performance.rs

- Coverage: 🟢 **98.4%** (60/61 lines)
- **Missed lines:**
  - Line 93

### crates/aetherd/src/migration/block.rs

- Coverage: 🟢 **100.0%** (10/10 lines)

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
