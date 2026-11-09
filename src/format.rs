use std::time::{SystemTime, UNIX_EPOCH};

/// YouTube-style short counts: 980, 7.9K, 106K, 1.2M.
pub fn compact(n: u64) -> String {
    const UNITS: [(u64, &str); 3] = [(1_000_000_000, "B"), (1_000_000, "M"), (1_000, "K")];
    let Some((size, suffix)) = UNITS.into_iter().find(|(size, _)| n >= *size) else {
        return n.to_string();
    };
    let whole = n / size;
    // YouTube truncates rather than rounds, and only shows a decimal below 10.
    let tenth = n % size * 10 / size;
    if whole < 10 && tenth > 0 {
        format!("{whole}.{tenth}{suffix}")
    } else {
        format!("{whole}{suffix}")
    }
}

pub fn views(n: u64) -> String {
    if n == 1 {
        "1 view".into()
    } else {
        format!("{} views", compact(n))
    }
}

/// `7:28`, `1:02:03`.
pub fn duration(secs: u32) -> String {
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// `11 months ago` for a Unix timestamp.
pub fn ago(timestamp: i64) -> String {
    const STEPS: [(i64, &str); 6] = [
        (365 * 86_400, "year"),
        (30 * 86_400, "month"),
        (7 * 86_400, "week"),
        (86_400, "day"),
        (3_600, "hour"),
        (60, "minute"),
    ];
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX));
    let elapsed = now.saturating_sub(timestamp).max(0);
    match STEPS.into_iter().find(|(size, _)| elapsed >= *size) {
        Some((size, unit)) => {
            let count = elapsed / size;
            let plural = if count == 1 { "" } else { "s" };
            format!("{count} {unit}{plural} ago")
        }
        None => "just now".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compacts_like_youtube() {
        assert_eq!(compact(980), "980");
        assert_eq!(compact(1_065), "1K");
        assert_eq!(compact(7_950), "7.9K");
        assert_eq!(compact(45_761), "45K");
        assert_eq!(compact(1_250_000), "1.2M");
        assert_eq!(compact(3_000_000_000), "3B");
    }

    #[test]
    fn durations() {
        assert_eq!(duration(17), "0:17");
        assert_eq!(duration(1_167), "19:27");
        assert_eq!(duration(3_723), "1:02:03");
    }
}
