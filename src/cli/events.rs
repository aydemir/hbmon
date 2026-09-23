use clap::Parser;
use serde_json::json;
use std::collections::HashSet;
use std::io::Write as _;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::ipc::send_request;
use crate::util::generate_uuid;

use super::resolve_sock;

#[derive(Debug, Parser)]
pub struct EventsArgs {
    #[arg(long)]
    pub sock: Option<PathBuf>,
    /// Virgüllü olay filtresi (örn. `exit,dep_missing`). Yoksa hepsi akar.
    #[arg(long)]
    pub event: Option<String>,
    /// Başlangıçta son N eşleşen olayı tekrar bas, sonra takibe geç.
    #[arg(long, default_value = "0")]
    pub tail: usize,
    /// Toplam izleme süresi sn (aşıda exit 124). Daemon bitene kadar
    /// beklenmek istenirse büyük değer verilir.
    #[arg(long, default_value = "300")]
    pub timeout: f64,
    #[arg(long, default_value = "500")]
    pub poll_ms: u64,
}

/// Olay akışı — deneysel (kilide uygun: daemon'a dokunmaz).
/// `log_tail` pull'unu stdout'a satır-satır akıtır; ajan bu komutu arka
/// planda çalıştırınca olaylar ona push edilmiş gibi düşer (TASK-013
/// wrapper-push deneyinin çekirdeğe dokunmayan devamı). Her satır ham
/// `.jsonl` olayıdır (`{"ts","v":1,"ev",...}`); `exit` olayı görülünce
/// build kodu ile çıkılır.
pub fn run(a: EventsArgs) -> Result<i32, String> {
    let sock = resolve_sock(a.sock)?;
    let filter = parse_filter(a.event.as_deref());
    let deadline = Instant::now() + Duration::from_secs_f64(a.timeout.max(1.0));
    let poll = Duration::from_millis(a.poll_ms.clamp(100, 10_000));

    // Replay: mevcut kuyruğun son N eşleşenini bas, konumu kuyruk sonuna al.
    let mut pos = StreamPos::default();
    if a.tail > 0 {
        let lines = fetch_tail(&sock, a.tail.saturating_mul(4).max(50))?;
        pos.advance(&lines);
        let kept: Vec<&String> = lines.iter().filter(|l| match_filter(l, &filter)).collect();
        let replay = if kept.len() <= a.tail {
            &kept[..]
        } else {
            &kept[kept.len() - a.tail..]
        };
        for l in replay {
            print_line(l);
        }
    }

    let mut stagnant = 0u32;
    loop {
        if Instant::now() >= deadline {
            eprintln!("{{\"v\":1,\"ev\":\"events_timeout\",\"timeout\":true}}");
            return Ok(124);
        }
        let lines = fetch_tail(&sock, 100)?;
        let start = pos.advance(&lines);
        if start >= lines.len() {
            stagnant += 1;
        } else {
            stagnant = 0;
        }
        for line in &lines[start..] {
            if !match_filter(line, &filter) {
                continue;
            }
            print_line(line);
            if let Some(code) = exit_code_of(line) {
                return Ok(code);
            }
        }
        // `shutdown` op'u `exit` olayı basmaz — daemon sessizce gider.
        // Yeni satır yokken ara ara compact durum yoklanır; terminal
        // state görülürse code ile çıkılır (ölü akışta takılma yok).
        if stagnant > 0 && stagnant.is_multiple_of(10) {
            if let Some(code) = terminal_code(&sock) {
                return Ok(code);
            }
        }
        std::thread::sleep(poll);
    }
}

fn fetch_tail(sock: &crate::platform::paths::SockAddr, n: usize) -> Result<Vec<String>, String> {
    let req = json!({"v":1,"op":"log_tail","id":generate_uuid(),"n":n});
    let resp = send_request(sock, &req, 10)?;
    if !resp.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        return Err(format!("log_tail failed: {}", resp));
    }
    Ok(resp
        .get("lines")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default())
}

/// Terminal state + code: `exit` olayı hiç basılmayan kapanışlar
/// (`shutdown`) için emniyet supabı. Terminal değilse `None`.
fn terminal_code(sock: &crate::platform::paths::SockAddr) -> Option<i32> {
    let req = json!({"v":1,"op":"status","id":generate_uuid(),"compact":true});
    let resp = send_request(sock, &req, 10).ok()?;
    if !resp.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        return None;
    }
    let terminal = matches!(
        resp.get("state").and_then(|v| v.as_str()).unwrap_or(""),
        "done" | "failed" | "dep_missing" | "oom_killed" | "timeout"
    );
    if !terminal {
        return None;
    }
    Some(
        resp.get("code")
            .and_then(|v| v.as_i64())
            .map(|c| c as i32)
            .unwrap_or(1),
    )
}

fn print_line(l: &str) {
    println!("{}", l);
    let _ = std::io::stdout().flush();
}

/// `until` ile aynı mini-dil: virgüllü liste → küme (boş = filtresiz).
fn parse_filter(event: Option<&str>) -> Option<HashSet<String>> {
    let set: HashSet<String> = event
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if set.is_empty() {
        None
    } else {
        Some(set)
    }
}

/// Filtre boşsa her satır geçer; bozuk JSON kaybolmasın diye aynen geçer.
fn match_filter(line: &str, filter: &Option<HashSet<String>>) -> bool {
    match filter {
        None => true,
        Some(set) => match serde_json::from_str::<serde_json::Value>(line) {
            Ok(v) => v
                .get("ev")
                .and_then(|e| e.as_str())
                .is_some_and(|ev| set.contains(ev)),
            Err(_) => true,
        },
    }
}

/// Akış konumu: görülen kuyruk suffix'inin son satırı + onunla biten
/// bitişik blok uzunluğu. `ts` saniye çözünürlüklü olduğundan bayt-aynı
/// komşu satırlar teorik olarak olabilir; salt "son satırı bul" mantığı
/// sınırda kalan ikinci kopyayı yutardı. Blok sayacı bitişik tekrarları
/// korur (append-only günlükte). Kalıntı teori: araya farklı satır giren
/// bayt-aynı çift (pratikte imkânsız — her olay ayırt edici alan taşır).
#[derive(Debug, Default)]
struct StreamPos {
    last: Option<String>,
    run: usize,
}

impl StreamPos {
    /// Kuyrukta görülmemişlerin başlangıç indeksi; konumu kuyruk sonuna
    /// taşır. `last` pencerede yoksa 0 — tekrar > kayıp.
    fn advance(&mut self, lines: &[String]) -> usize {
        let start = match &self.last {
            Some(lp) => match lines.iter().rposition(|l| l == lp) {
                Some(i) => {
                    let mut block = 1;
                    while block <= i && lines[i - block] == *lp {
                        block += 1;
                    }
                    // Bloktan görülen kuyrukla örtüşen düşer, fazlası taze.
                    i + 1 - (block - self.run.min(block))
                }
                None => 0,
            },
            None => 0,
        };
        if let Some(l) = lines.last() {
            let mut run = 1;
            while run < lines.len() && lines[lines.len() - 1 - run] == *l {
                run += 1;
            }
            self.last = Some(l.clone());
            self.run = run;
        }
        start
    }
}

/// `exit` olayı → build kodu; başka satır → `None` (akış sürer).
fn exit_code_of(line: &str) -> Option<i32> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    if v.get("ev").and_then(|e| e.as_str()) != Some("exit") {
        return None;
    }
    Some(
        v.get("code")
            .and_then(|c| c.as_i64())
            .map(|c| c as i32)
            .unwrap_or(1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_parses_csv_and_empty_is_none() {
        assert_eq!(parse_filter(None), None);
        assert_eq!(parse_filter(Some("")), None);
        assert_eq!(parse_filter(Some("  ")), None);
        let f = parse_filter(Some("exit, dep_missing")).unwrap();
        assert!(f.contains("exit") && f.contains("dep_missing"));
    }

    #[test]
    fn filter_matches_ev_and_keeps_broken_lines() {
        let f = parse_filter(Some("exit"));
        assert!(match_filter(r#"{"v":1,"ev":"exit","code":0}"#, &f));
        assert!(!match_filter(r#"{"v":1,"ev":"metric"}"#, &f));
        assert!(match_filter("bozuk satır", &f));
        assert!(match_filter(r#"{"v":1,"ev":"metric"}"#, &None));
    }

    #[test]
    fn stream_resumes_after_last_seen() {
        let mut pos = StreamPos::default();
        assert_eq!(pos.advance(&[]), 0);
        let lines = vec!["a".to_string(), "b".to_string()];
        assert_eq!(pos.advance(&lines), 0);
        // Hepsi görüldü: aynı kuyrukta yeni yok.
        assert_eq!(pos.advance(&lines), 2);
        // Yeni satır arkaya eklenir.
        let grown = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(pos.advance(&grown), 2);
        // Pencere dışına düştü → baştan (kayıp yok).
        assert_eq!(pos.advance(&["zzz".to_string()]), 0);
    }

    #[test]
    fn stream_keeps_adjacent_duplicates() {
        // Sınırda kalan bitişik kopya yutulmaz: [.., L] görüldü,
        // kuyruk [.., L, L] oldu → ikinci L taze.
        let mut pos = StreamPos::default();
        pos.advance(&["a".to_string(), "L".to_string()]);
        let tail = vec!["a".to_string(), "L".to_string(), "L".to_string()];
        assert_eq!(pos.advance(&tail), 2);
        // Üçüz: görülen blok 2, kuyruk 3 → sonuncusu taze.
        let tail3 = vec!["L".to_string(), "L".to_string(), "L".to_string()];
        assert_eq!(pos.advance(&tail3), 2);
        // Blok tamamen görüldü → sessizlik.
        assert_eq!(pos.advance(&tail3), 3);
    }

    #[test]
    fn exit_line_yields_code_others_none() {
        assert_eq!(exit_code_of(r#"{"v":1,"ev":"exit","code":2}"#), Some(2));
        assert_eq!(exit_code_of(r#"{"v":1,"ev":"metric"}"#), None);
        assert_eq!(exit_code_of("bozuk"), None);
    }

    /// TASK-050: `summary`'li exit (yeni) ve summary'siz exit (eski log)
    /// aynı kodu verir — alan tamamen opsiyonel.
    #[test]
    fn exit_line_with_or_without_summary_yields_code() {
        assert_eq!(
            exit_code_of(r#"{"v":1,"ev":"exit","code":0,"summary":"tail…"}"#),
            Some(0)
        );
        assert_eq!(
            exit_code_of(r#"{"v":1,"ev":"exit","uuid":"u","pid":1,"code":1,"state":"failed"}"#),
            Some(1)
        );
    }
}
