//! Daemon signal posture + sinyal adı çözümleme.
//!
//! Gerçekleşim `crate::platform::signal` altında (multi-OS ilkesi:
//! OS kodu burada değil, platform katmanında). Bu modül ince uyumluluk
//! shimi — mevcut `crate::daemon::signals::*` importları çalışır.

pub use crate::platform::signal::{daemon_posture as install_daemon_posture, parse_signal, Sig};
