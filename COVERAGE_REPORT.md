# Coverage Report

**1413** / **1698** lines covered — **83.22%**

**Threshold:** 80% | **Status:** ✅ PASS

| Crate | Files | Covered | Total | Coverage |
| --- | --- | --- | --- | --- |
| `aether-aggregator` | 8 | 372 | 454 | 81.9% |
| `aether-auth` | 3 | 175 | 189 | 92.6% |
| `aether-fence` | 1 | 0 | 2 | 0.0% |
| `aetherd` | 15 | 866 | 1053 | 82.2% |

## `aether-aggregator`

### crates/aether-aggregator/src/main.rs

- Coverage: 🔴 **0.0%** (0/24 lines)

- **Missed lines:**
  - Lines 19-22
  - Lines 25-26
  - Lines 29-30
  - Line 32
  - Lines 35-37
  - Lines 39-43
  - Line 48
  - Lines 50-54
  - Line 56

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

### crates/aether-aggregator/tests/heartbeat_timeout_tests.rs

- Coverage: 🟢 **94.3%** (50/53 lines)

- **Missed lines:**
  - Line 34
  - Line 44
  - Line 65

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


## `aether-fence`

### crates/aether-fence/src/main.rs

- Coverage: 🔴 **0.0%** (0/2 lines)

- **Missed lines:**
  - Lines 7-8

## `aetherd`

### crates/aetherd/src/main.rs

- Coverage: 🔴 **0.0%** (0/43 lines)

- **Missed lines:**
  - Lines 20-23
  - Line 25
  - Lines 27-29
  - Line 32
  - Lines 35-36
  - Lines 39-41
  - Lines 43-44
  - Lines 49-53
  - Lines 56-63
  - Line 67
  - Lines 70-71
  - Line 73
  - Lines 75-80
  - Line 82
  - Line 84
  - Lines 87-88

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
  - Line 146
  - Lines 178-181
  - Lines 189-190
  - Line 192
  - Lines 196-197
  - Lines 203-206
  - Lines 208-209
  - Line 212
  - Line 260

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

### crates/aetherd/tests/vsock_stream_performance.rs

- Coverage: 🟢 **98.4%** (60/61 lines)

- **Missed lines:**
  - Line 93

### crates/aetherd/src/vsock.rs

- Coverage: 🟢 **100.0%** (79/79 lines)


### crates/aetherd/tests/bidding_resource_thresholds.rs

- Coverage: 🟢 **100.0%** (34/34 lines)


### crates/aetherd/tests/host_system_metrics_query.rs

- Coverage: 🟢 **100.0%** (15/15 lines)


### crates/aetherd/tests/qemu_kvm_vm_lifecycle.rs

- Coverage: 🟢 **100.0%** (48/48 lines)

