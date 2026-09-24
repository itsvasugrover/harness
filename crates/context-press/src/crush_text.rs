//! Text crusher: collapse consecutive repeats with counts, keep
//! ERROR/FATAL lines byte-identical, drop nothing else.
pub fn crush_text(text: &str) -> String {
    let mut out = Vec::new();
    let mut prev: Option<&str> = None;
    let mut run = 0usize;
    let flush = |line: Option<&str>, n: usize, dst: &mut Vec<String>| {
        if let Some(l) = line {
            if n > 2 {
                dst.push(format!("{l}  [x{n}]"));
            } else {
                for _ in 0..n {
                    dst.push(l.to_string());
                }
            }
        }
    };
    for line in text.lines() {
        let important =
            line.to_lowercase().contains("error") || line.to_lowercase().contains("fatal");
        if important {
            flush(prev.take(), run, &mut out);
            run = 0;
            out.push(line.to_string());
            continue;
        }
        match prev {
            Some(p) if p == line => run += 1,
            _ => {
                flush(prev, run, &mut out);
                prev = Some(line);
                run = 1;
            }
        }
    }
    flush(prev, run, &mut out);
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_runs_keeps_fatal() {
        let text = "ok\nok\nok\nok\nFATAL boom\nok\nok\nok\nok";
        let crushed = crush_text(text);
        assert!(crushed.contains("[x4]"));
        assert!(crushed.contains("FATAL boom"));
    }
}
