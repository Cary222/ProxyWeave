# Third-Party Notices & Open Source Attributions

ProxyWeave (编织网关) incorporates and references ideas, architecture, and code from the following open-source projects under permissive open-source licenses. We gratefully acknowledge the original authors and maintainers.

---

### 1. Quotio Desktop
- **Repository**: https://github.com/xiaocoss/quotio-desktop
- **License**: MIT License
- **Copyright**: Copyright (c) 2024-2026 xiaocoss
- **Usage in ProxyWeave**: Main Desktop & Platform Scaffold (Tauri v2 shell, system tray, platform abstraction in `quotio-platform`, IDE configuration reader in `agent_config.rs`, asset download utilities).

---

### 2. TiyGate
- **Repository**: https://github.com/tiylabs/tiygate
- **License**: Apache License 2.0
- **Copyright**: Copyright (c) 2024-2026 TIY Labs
- **Usage in ProxyWeave**: Protocol Codec IR, OpenAI Chat Completions, Responses API, and Anthropic Messages stream state machine reference.

---

### 3. Tunnel Agent
- **Repository**: https://github.com/beyondhumane/tunnel-agent
- **License**: MIT License
- **Copyright**: Copyright (c) 2024-2026 Beyond Humane & Villoh
- **Usage in ProxyWeave**: Multi-engine management UX, version pinning workflow, Win32 Job Object test patterns.

---

### 4. DeepSeek Harness (DSH)
- **Repository**: https://github.com/deepseek-ai/deepseek-harness
- **License**: MIT License
- **Copyright**: Copyright (c) 2025 DeepSeek AI
- **Usage in ProxyWeave**: Cordis microkernel philosophy ("Everything is a plugin"), reversible effects lifecycle, minimal dark industrial UI.

---

### 5. shared-child
- **Repository**: https://github.com/oconnor663/shared-child.rs
- **License**: MIT / Apache-2.0
- **Usage in ProxyWeave**: Process supervisor reference for child process synchronization and kill race prevention.

---

### 6. portpicker-rs
- **Repository**: https://github.com/Cargo-Prebuilt/portpicker-rs
- **License**: MIT / Apache-2.0
- **Usage in ProxyWeave**: Dynamic non-colliding loopback port allocation algorithm.
