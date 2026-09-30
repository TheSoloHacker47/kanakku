//! Number formatting the way Indian readers expect it.

use crate::i18n::Lang;

/// `12345678` becomes `₹1,23,45,678`.
pub fn inr(amount: i64) -> String {
    let digits = amount.unsigned_abs().to_string();
    let (head, last3) = digits.split_at(digits.len().saturating_sub(3));
    let mut out = String::with_capacity(digits.len() + digits.len() / 2 + 4);
    if amount < 0 {
        out.push('-');
    }
    out.push('₹');
    for (i, c) in head.chars().enumerate() {
        if i > 0 && (head.len() - i) % 2 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if !head.is_empty() {
        out.push(',');
    }
    out.push_str(last3);
    out
}

/// A short form for lists: `₹21.43 crore`, `₹5.2 lakh`, or the full figure below a lakh.
pub fn inr_short(amount: i64, lang: Lang) -> String {
    const CRORE: i64 = 10_000_000;
    const LAKH: i64 = 100_000;
    let (unit, label) = match amount.abs() {
        a if a >= CRORE => (CRORE, lang.pick("കോടി", "crore")),
        a if a >= LAKH => (LAKH, lang.pick("ലക്ഷം", "lakh")),
        _ => return inr(amount),
    };
    // Hundredths of the unit, so the whole part can be grouped like any other rupee figure.
    let hundredths = (amount as f64 / unit as f64 * 100.0).round() as i64;
    let whole = inr(hundredths / 100);
    let fraction = format!("{:02}", hundredths.abs() % 100);
    let fraction = fraction.trim_end_matches('0');
    if fraction.is_empty() {
        format!("{whole} {label}")
    } else {
        format!("{whole}.{fraction} {label}")
    }
}

/// `60.23879955` becomes `60.2%`, `80` becomes `80%`.
pub fn pct(value: f64) -> String {
    let rounded = (value * 10.0).round() / 10.0;
    if rounded.fract() == 0.0 {
        format!("{rounded:.0}%")
    } else {
        format!("{rounded:.1}%")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indian_digit_grouping() {
        assert_eq!(inr(0), "₹0");
        assert_eq!(inr(999), "₹999");
        assert_eq!(inr(1_000), "₹1,000");
        assert_eq!(inr(99_999), "₹99,999");
        assert_eq!(inr(100_000), "₹1,00,000");
        assert_eq!(inr(12_345_678), "₹1,23,45,678");
        assert_eq!(inr(5_665_100_000), "₹5,66,51,00,000");
        assert_eq!(inr(-1_500), "-₹1,500");
    }

    #[test]
    fn short_amounts() {
        assert_eq!(inr_short(214_300_000, Lang::En), "₹21.43 crore");
        assert_eq!(inr_short(214_300_000, Lang::Ml), "₹21.43 കോടി");
        assert_eq!(inr_short(5_665_100_000, Lang::En), "₹566.51 crore");
        assert_eq!(inr_short(10_000_000, Lang::En), "₹1 crore");
        assert_eq!(inr_short(520_000, Lang::En), "₹5.2 lakh");
        assert_eq!(inr_short(99_999, Lang::En), "₹99,999");
        assert_eq!(inr_short(16_531_359_995, Lang::En), "₹1,653.14 crore");
        assert_eq!(inr_short(28_325_500_000, Lang::En), "₹2,832.55 crore");
        assert_eq!(inr_short(9_999_999, Lang::En), "₹100 lakh");
    }

    #[test]
    fn percentages() {
        assert_eq!(pct(80.0), "80%");
        assert_eq!(pct(60.23879955), "60.2%");
        assert_eq!(pct(99.96), "100%");
        assert_eq!(pct(0.0), "0%");
    }
}
