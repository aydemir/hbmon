---
name: Bug report
description: hbmon beklediğin gibi davranmadıysa doldur
labels: [bug]
body:
  - type: markdown
    attributes:
      value: |
        Önce bilinen limitation'ları ele (hepsi `README.md` → Platform support):
        - [ ] Windows named pipe'ta ACL kilidi yok (`no ACL lockdown`) — multi-user makinede başkasının build'i görülebilir/öldürülebilir.
        - [ ] cgroup reaping HBMon kontrolünde değil ([HBMON-RFC.md](../../HBMON-RFC.md) §4.2.1) — "niye öldü?" önce host/harness politikasına bak.
        - [ ] macOS CPU best-effort; ağ sayımı yok.
        - [ ] Windows cmdline = exe yolu (tam komut satırı yok).
        Yukarıdakilerden biriyse bu bug değil, belgeli eksiktir.
  - type: input
    id: os
    attributes:
      label: OS + mimari
      placeholder: "örn. ubuntu-24.04 x86_64 / macos-arm64 / windows-latest"
    validations:
      required: true
  - type: input
    id: version
    attributes:
      label: hbmon sürümü
      placeholder: "örn. v0.1.1 (crates.io) / master@2c76def"
    validations:
      required: true
  - type: textarea
    id: command
    attributes:
      label: Çalıştırılan komut
      placeholder: "örn. hbmon watch --detach -- cargo build"
    validations:
      required: true
  - type: textarea
    id: compact
    attributes:
      label: `hbmon status --sock $SOCK --compact` çıktısı
      description: Mümkünse tam JSON'u yapıştır.
    validations:
      required: false
  - type: textarea
    id: logtail
    attributes:
      label: `hbmon log --sock $SOCK --tail 30` çıktısı (ilgili kısım)
    validations:
      required: false
  - type: textarea
    id: expected
    attributes:
      label: Beklenen vs gerçekleşen
    validations:
      required: true
